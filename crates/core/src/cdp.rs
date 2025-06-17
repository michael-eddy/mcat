use base64::Engine;
use base64::engine::general_purpose;
use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

use crate::fetch_manager::BrowserConfig;

pub struct ChromeHeadless {
    process: Child,
    port: u16,
}

impl ChromeHeadless {
    pub async fn new(uri: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let browser_config = BrowserConfig::default().ok_or("chromium isn't installed. either install it manually (chrome/msedge will do so too) or call `mcat --fetch-chromium`")?;
        let path = browser_config.path;
        let process = Command::new(path)
            .args(&[
                // Core headless setup
                "--headless",
                "--disable-gpu",
                "--remote-debugging-port=9222",
                // Stability & crash prevention
                "--disable-dev-shm-usage",
                "--disable-breakpad",
                "--disable-hang-monitor",
                // Disable unnecessary features
                "--disable-extensions",
                "--disable-plugins",
                "--disable-default-apps",
                "--disable-sync",
                "--disable-background-networking",
                "--disable-features=TranslateUI",
                "--disable-features=VizDisplayCompositor",
                // UI/UX optimizations
                "--no-first-run",
                "--disable-popup-blocking",
                "--disable-prompt-on-repost",
                // Performance optimizations
                "--disable-background-timer-throttling",
                "--disable-renderer-backgrounding",
                "--disable-backgrounding-occluded-windows",
                "--disable-ipc-flooding-protection",
                "--metrics-recording-only",
                // Memory & rendering
                "--memory-pressure-off",
                "--max_old_space_size=4096",
                "--force-color-profile=srgb",
                uri,
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let instance = Self {
            process,
            port: 9222,
        };

        instance.wait_for_server().await?;
        Ok(instance)
    }

    async fn wait_for_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            match timeout(
                Duration::from_millis(100),
                TcpStream::connect(format!("127.0.0.1:{}", self.port)),
            )
            .await
            {
                Ok(Ok(_)) => {
                    return Ok(());
                }
                Ok(Err(_)) | Err(_) => {
                    tokio::task::yield_now().await;
                }
            }
        }
    }

    pub async fn capture_screenshot(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let endpoint = self.get_websocket_endpoint().await?;
        let (mut ws_stream, _) = tokio_tungstenite::connect_async(&endpoint).await?;

        // Get the page ready
        self.send_command(&mut ws_stream, 1, "Page.enable", json!({}))
            .await?;
        self.wait_for_load_event(&mut ws_stream).await?;

        //  Get layout metrics
        let metrics = self
            .send_command(&mut ws_stream, 2, "Page.getLayoutMetrics", json!({}))
            .await?;
        let width = metrics["contentSize"]["width"].as_f64().unwrap();
        let height = metrics["contentSize"]["height"].as_f64().unwrap();

        // Set viewport
        self.send_command(
            &mut ws_stream,
            3,
            "Emulation.setDeviceMetricsOverride",
            json!({
                "mobile": false,
                "width": width,
                "height": height,
                "deviceScaleFactor": 1
            }),
        )
        .await?;

        // Capture screenshot
        let response = self
            .send_command(
                &mut ws_stream,
                4,
                "Page.captureScreenshot",
                json!({
                    "format": "png",
                }),
            )
            .await?;

        let screenshot_data = response["data"]
            .as_str()
            .ok_or("failed to get screenshot")?;
        Ok(general_purpose::STANDARD.decode(screenshot_data)?)
    }

    async fn get_websocket_endpoint(&self) -> Result<String, Box<dyn std::error::Error>> {
        // shouldn't really go over 1 and even
        let max_attempts = 10;
        for _ in 1..=max_attempts {
            let url = format!("http://localhost:{}/json", self.port);
            let body = reqwest::get(&url).await?.text().await?;
            let json: Value = serde_json::from_str(&body)?;
            match json[0]["webSocketDebuggerUrl"].as_str() {
                Some(s) => return Ok(s.to_owned()),
                None => {
                    // sometimes its too early I guess
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            };
        }
        Err("Failed to get wesocket for headless chrome".into())
    }

    async fn send_command(
        &self,
        ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
        id: u64,
        method: &str,
        params: Value,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let command = json!({
            "id": id,
            "method": method,
            "params": params
        });

        ws_stream
            .send(Message::Text(command.to_string().into()))
            .await?;

        while let Some(msg) = ws_stream.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                let response: Value = serde_json::from_str(&text)?;
                if response["id"] == id {
                    if let Some(error) = response.get("error") {
                        return Err(format!("Chrome error: {}", error).into());
                    }
                    return Ok(response["result"].clone());
                }
            }
        }

        Err("WebSocket connection closed unexpectedly".into())
    }

    async fn wait_for_load_event(
        &self,
        ws_stream: &mut WebSocketStream<MaybeTlsStream<TcpStream>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use futures::stream::StreamExt;

        while let Some(msg) = ws_stream.next().await {
            let msg = msg?;
            if let Message::Text(text) = msg {
                if let Ok(json) = serde_json::from_str::<Value>(&text) {
                    if json["method"] == "Page.loadEventFired" {
                        return Ok(());
                    }
                }
            }
        }

        Err("WebSocket closed before loadEventFired".into())
    }
}

impl Drop for ChromeHeadless {
    fn drop(&mut self) {
        let _ = self.process.kill();
    }
}
