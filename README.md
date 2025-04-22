<div align="center">
  
<img src="https://github.com/user-attachments/assets/d6dddc88-2f75-44c3-9b1d-04ea5651c35d" width="128"/>


# Mcat
![Downloads](https://img.shields.io/crates/d/mcat?style=for-the-badge) ![Version](https://img.shields.io/crates/v/mcat?style=for-the-badge)

![mcat_demo](https://github.com/user-attachments/assets/607c1a41-af3b-428e-a8d8-c30ac8c5e884)
</div>

## ✨ Features
* 📄 **File to Markdown/HTML**  
  Convert structured content like CSVs, directories, and rich document formats (e.g., DOCX) into clean Markdown/HTML.
  
* 🏞️ **Markdown/HTML to Image**  
  Render Markdown or HTML files into images.
  
* 🖼️ **Inline Image/Videos**  
  Display images/videos *inside* your terminal using protocols like Kitty, iTerm, or Sixel.
  
* 🌐 **URL to Inline Image/Video**  
  View Images/Videos from a URL in your terminal
  
* 💃🏻 **Automatic Styling for HTML**  
  automatically inject styles into the HTML to make the image cooler!
  ```sh
  # dark theme (default is light theme)
  mcat somefile.md -i -theme makurai
  # or for short ~
  mcat somefile.md -im
  # or you own theme ~
  mcat somefile.md -i -t "somefile.css"
  ```

## ⬇️ Installation
```sh
cargo install mcat
```
or ~
```sh
git clone https://github.com/Skardyy/mcat
cd mcat
cargo install --path .
```
(Coming soon: prebuilt binaries for common platforms.)

## 🏋️ Example Usage
```sh
# View a PDF as Markdown
mcat resume.pdf

# Render Markdown to an image
mcat notes.md -i

# Show an image inline in your terminal
mcat diagram.png -i

# Save a document as image
mcat document.docx -o image > img.png

# from a url
mcat "https://giphy.com/gifs/..."
```

## ⚙️ Supported Formats
| Input Type | Output Options |
|---|---|
| DOCX, PDF, CSV, ODT, PPTX, and more.. | Markdown, HTML, Image, Inline |
| Markdown / HTML | Image, Inline Image |
| Images, Videos | Inline Display |
| URLs | Image/Video Fetch + Inline View |

## 🛐 Dependencies
Mcat tries to have as little dependencies as possible.
#### chromium (for rendering HTML to image):
1. exists on every windows machine through msedge.
2. auto installs the binaries if missing
#### ffmpeg (for videos)
1. auto installs binaries if missing

## 🆘 Help
```sh
mcat --help
Usage: mcat.exe [OPTIONS] <input>

Arguments:
  <input>  file / dir

Options:
  -o <output>                            the format to output [possible values: html, md, image, inline]
  -t <theme>                             alternative css file for images, valid options: [default, makurai, <local file>] [default: default]
  -s                                     add style to html too (when html is the output)
      --inline-options <inline-options>  options for the --output inline
                                         *  center=<bool>
                                         *  width=<string> [only for images]
                                         *  height=<string> [only for images]
                                         *  spx=<string>
                                         *  sc=<string>
                                         *  zoom=<usize> [doesn't work yet]
                                         *  x=<int> [doesn't work yet]
                                         *  y=<int> [doesn't work yet]
                                         *  exmp: --inline-options 'center=false,width=80%,height=20c,spx=1920x1080,sc=100x20,zoom=2,x=16,y=8'
      --kitty                            makes the inline image encoded to kitty
      --iterm                            makes the inline image encoded to iterm
      --sixel                            makes the inline image encoded to sixel
  -r, --raw                              allows raw html to run (put only on your content)
  -i                                     shortcut for putting --output inline
  -m                                     shortcut for putting --theme makurai
  -h, --help                             Print help
  -V, --version                          Print version
```

## 🚧 Roadmap
- [ ] mcat.nvim: a neovim plugin to use mcat inside neovim

## 📎 License
MIT License
