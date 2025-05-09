<div align="center" markdown="1">
   <sup>Special thanks to:</sup>
   <br>
   <br>
   <a href="https://www.warp.dev/mcat">
      <img alt="Warp sponsorship" width="400" src="https://github.com/user-attachments/assets/c21102f7-bab9-4344-a731-0cf6b341cab2">
   </a>

### [Warp, the intelligent terminal for developers](https://www.warp.dev/mcat)
[Available for MacOS, Linux, & Windows](https://www.warp.dev/mcat)<br>

</div>
<hr>

<div align="center">
  
<img src="https://i.imgur.com/qSSM6Iy.png" width="128"/>


# Mcat
![Downloads](https://img.shields.io/crates/d/mcat?style=for-the-badge) ![Version](https://img.shields.io/crates/v/mcat?style=for-the-badge)  

[Installation](#%EF%B8%8F-installation) • [Examples](#%EF%B8%8F-example-usage) • [CHANGELOG](./CHANGELOG.md)

![mcat_demo](https://github.com/user-attachments/assets/b47aa276-f0e4-4259-b2c5-1525d7d9d6cb)
</div>

## ✨ Features
* 📄 **File to Markdown/HTML**  
  Convert structured content like CSVs, directories, and rich document formats (e.g., DOCX) into clean Markdown/HTML.
  
* 🏞️ **Markdown/HTML to Image**  
  Render Markdown or HTML files into images.
  
* 🖼️ **Inline Image/Videos**  
  Display images/videos *inside* your terminal using protocols like Kitty, iTerm, or Sixel.
  
* 🌐 **Handles URLs too!**  
  View Images/Videos/Document from a URL in your terminal (or save them)

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
cargo install --path ./crates/core
```
or prebuilt from the [latest release](https://github.com/Skardyy/mcat/releases/latest)

## 🏋️ Example Usage
```sh
# View a document as Markdown
mcat resume.pdf

# View a document as pretty text in the terminal
mcat resume.pdf -p

# Or from a url!
mcat "https://somewebite.com/file.pdf"

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

# From stdin?
mcat < somefile.png

# Concatenate documents and turn them into an image
mcat document.docx presentation.odt table.xlsx archive.zip -o image > all.png

# Or just select interactively and copy to clipboard (for ai prompts)
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
| Markdown | Pretty terminal formatting |
| Markdown / HTML | Image, Inline Image |
| Images, Videos | Inline Display |
| URLs | Fetch + Everything above! |

## 🛐 Dependencies
Mcat tries to have as little dependencies as possible.
#### chromium (for rendering HTML to image):
1. exists on every windows machine through msedge. and other machines that have chrome/msedge/chromium
2. can be installed by doing `mcat --fetch-chormium`
#### ffmpeg (for videos)
1. if your machine has it 🫠.
2. can be installed by doing `mcat --fetch-ffmpeg`

## 🆘 Help
```txt
mcat --help
Usage: mcat.exe [OPTIONS] [input]...

Arguments:
  [input]...  file / dir / url

Options:
  -o <output>                            the format to output [possible values: html, md, pretty, image, video, inline]
  -t <theme>                             alternative css file for images, valid options: [default, makurai, <local file>] [default: default]
  -s                                     add style to html too (when html is the output)
      --kitty                            makes the inline image encoded to kitty
      --iterm                            makes the inline image encoded to iterm
      --sixel                            makes the inline image encoded to sixel
  -i                                     shortcut for putting --output inline
  -p                                     shortcut for putting --output pretty
  -m                                     shortcut for putting --theme makurai
      --hori                             concat images horizontal instead of vertical
      --inline-options <inline-options>  options for the --output inline
                                         *  center=<bool>
                                         *  width=<string> [only for images]
                                         *  height=<string> [only for images]
                                         *  scale=<f32>
                                         *  spx=<string>
                                         *  sc=<string>
                                         *  zoom=<usize> [only for images]
                                         *  x=<int> [only for images]
                                         *  y=<int> [only for images]
                                         *  exmp: --inline-options 'center=false,width=80%,height=20c,scale=0.5,spx=1920x1080,sc=100x20,zoom=2,x=16,y=8'
      --fetch-chromium                   download and prepare chromium
      --fetch-ffmpeg                     download and prepare ffmpeg
      --fetch-clean                      Clean up the local binaries
  -h, --help                             Print help
  -V, --version                          Print version
```

## 🚧 Roadmap
- [ ] mcat.nvim: a neovim plugin to use mcat inside neovim

## 📎 License
MIT License
