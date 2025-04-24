<div align="center">
  
<img src="https://github.com/user-attachments/assets/d6dddc88-2f75-44c3-9b1d-04ea5651c35d" width="256"/>


# Mcat
![Downloads](https://img.shields.io/crates/d/mcat?style=for-the-badge) ![Version](https://img.shields.io/crates/v/mcat?style=for-the-badge)  

[Installation](#%EF%B8%8F-installation) • [Examples](#%EF%B8%8F-example-usage) • [CHANNELOG](./CHANNELOG.md)

![mcat_demo](https://github.com/user-attachments/assets/b47aa276-f0e4-4259-b2c5-1525d7d9d6cb)
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

* 🔗 **Concatenate Images and Video too!**  
  Concatenate videos of the same format (time concat)
  and Concatenate images by stacking them horizontal or vertical(default)
  
* 💃🏻 **Automatic Styling for HTML**  
  automatically inject styles into the HTML to make the image cooler!

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
or prebuilt from the [latest release](https://github.com/Skardyy/mcat/releases/latest)

## 🏋️ Example Usage
```sh
# View a document as Markdown
mcat resume.pdf

# Or HTML
mcat project.docx -o html

# Show a document inline as an image
mcat readme.md -i

# Show a document as an image inline with a dark theme
mcat presentation.pptx -im

# Show a document as an image in the terminal with your own css
mcat document.pdf -it "path/to/your/file.css"

# Render a document to an image and save it
mcat readme.md -o image > img.png

# Show media inline in your terminal
mcat diagram.png
mcat video.mp4

# From a url
mcat "https://giphy.com/gifs/..."

# Images too!
mcat "https://website/images/..."

# Concatenate documents and turn them into an image
mcat document.docx presentation.odt table.xlsx archive.zip -o image > all.png

# Or just select interactively and copy to clipboard
# Replace scb with a command from your os
mcat directory | scb

# Concatenate images (stacks vertical)
mcat someimage.png anotherimage.bmp

# Or save it (stacks horizontal)
mcat someimage.png anotherimage.bmp --hori -o image > save.png

# Concatenate videos (must be same format: codec,audio..)
mcat part1.mp4 anothervideo.mp4 -o video > save.mp4
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
```txt
mcat --help
Usage: mcat.exe [OPTIONS] <input>...

Arguments:
  <input>...  file / dir

Options:
  -o <output>                            the format to output [possible values: html, md, image, video, inline]
  -t <theme>                             alternative css file for images, valid options: [default, makurai, <local file>] [default: default]
  -s                                     add style to html too (when html is the output)
      --kitty                            makes the inline image encoded to kitty
      --iterm                            makes the inline image encoded to iterm
      --sixel                            makes the inline image encoded to sixel
  -r, --raw                              allows raw html to run (put only on your content)
  -i                                     shortcut for putting --output inline
  -m                                     shortcut for putting --theme makurai
      --hori                             concat images horizontal instead of vertical
      --inline-options <inline-options>  options for the --output inline
                                         *  center=<bool>
                                         *  width=<string> [only for images]
                                         *  height=<string> [only for images]
                                         *  scale=<f32>
                                         *  spx=<string>
                                         *  sc=<string>
                                         *  zoom=<usize> [doesn't work yet]
                                         *  x=<int> [doesn't work yet]
                                         *  y=<int> [doesn't work yet]
                                         *  exmp: --inline-options 'center=false,width=80%,height=20c,scale=0.5,spx=1920x1080,sc=100x20,zoom=2,x=16,y=8'
  -h, --help                             Print help
  -V, --version                          Print version
```

## 🚧 Roadmap
- [ ] mcat.nvim: a neovim plugin to use mcat inside neovim

## 📎 License
MIT License
