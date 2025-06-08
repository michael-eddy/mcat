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

![mcat_demo](https://github.com/Skardyy/assets/blob/main/mcat_opt.gif)
</div>

## ✨ Features
* 📄 **File to Markdown/HTML**  
  Convert structured content like CSVs, directories, and rich document formats (e.g., DOCX) into clean Markdown/HTML.
  
* 🏞️ **Markdown/HTML to Image**  
  Render Markdown or HTML files into images.
  
* 🖼️ **Inline Image/Videos**  
  Display images/videos *inside* your terminal using protocols like Kitty, iTerm, or Sixel (with tmux support!).
  
* 🌐 **Handles URLs and Stdin too!**  
  You don't have to save things locally in your PC to use mcat!

* 🔗 **Concatenate Images and Video too!**  
  Concatenate videos of the same format (time concat)
  and Concatenate images by stacking them horizontal or vertical(default)
  
* 💃🏻 **Style-able**  
  Contains different themes to fit your taste

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

## ⚙️ Supported Pipeline
![mcat-pipeline](https://github.com/user-attachments/assets/fbf4617d-453a-45e8-bbd5-5dfdac2b8086)


## 🏋️ Example Usage
```sh
# View a document at the terminal
mcat resume.pdf

# Or from a url!
mcat "https://somewebite.com/file.pdf"

# Need it as HTML?
mcat project.docx -o html

# list a directory using images!
mcat ls

# Show a document inline as an image
mcat readme.md -i

# Or HTML!
mcat index.html -i

# Just save?
mcat index.html -o image > page.png

# Too big? zoom and pan it inside the terminal!
mcat file1.js file2.js file3.js file4.js -o interactive
mcat bigimg.png -o interactive

# Show a document as an image inline with a different theme
mcat main.rs another.rs -it monokai

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

## 🛐 Dependencies
Mcat tries to have as few dependencies as possible.
<details>
<summary><strong>Chromium (for rendering HTML to image)</strong></summary>

```md
1. Available by default on most Windows machines via Microsoft Edge.
2. Also works with any installed Chrome, Edge, or Chromium.
3. You can install it manually via `mcat --fetch-chromium`
```
</details>

<details>
<summary><strong>FFmpeg (for videos)</strong></summary>

```md
1. If it's already on your machine 🫠.
2. Otherwise, you can install it with `mcat --fetch-ffmpeg`
```
</details>

## ⚙️ Configuring
<details>
<summary><strong>Using Flags</strong></summary>

```md
the main flags for configuring are:
* `--opts` for inline image printing
* `--ls-opts` for the ls command

run `mcat --help` for full detail, and other flags. 
```
</details>

<details>
<summary><strong>Using Environment Variables</strong></summary>

```md
each variable mimicks its corresponding flag alternative.
* `MCAT_ENCODER`, Options: kitty,iterm,sixel,ascii. e.g. MCAT_ENCODER=kitty is the same as doing `--kitty`
* `MCAT_THEME`, <str> same as the `--theme` flag
* `MCAT_INLINE_OPTS`, <str> same as the `--opts` flag
* `MCAT_LS_OPTS`, <str> same as the `--ls-opts` flag
* `MCAT_SILENT`, <bool> same as the `--silent` flag
* `MCAT_NO_LINENUMBERS`, <bool> same as the `--no-linenumbers` flag
```
</details>


## 🚧 Roadmap
- [ ] mcat.nvim: a neovim plugin to use mcat inside neovim

## 📎 License
MIT License
