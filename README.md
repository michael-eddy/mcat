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

# Mcat

<img src="https://i.imgur.com/qSSM6Iy.png" width="128"/>

parse, convert and preview files  
***in your terminal***

![Downloads](https://img.shields.io/crates/d/mcat?style=for-the-badge) ![Version](https://img.shields.io/crates/v/mcat?style=for-the-badge)  

[Installation](#%EF%B8%8F-installation) • [Examples](#%EF%B8%8F-example-usage) • [CHANGELOG](./CHANGELOG.md)

![mcat_demo](https://github.com/Skardyy/assets/blob/main/mcat_opt.gif)
</div>

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

## ⚙️ How it works

![mcat-pipeline](https://github.com/user-attachments/assets/fbf4617d-453a-45e8-bbd5-5dfdac2b8086)

<details>
<summary>Advanced explanation</summary>
   
---


| Input |
|-------|

Inputs can be:
1. local file
2. url
3. bytes from stdin

The type of each input is inferred automatically, and it continues through the pipeline until it reaches the output format the user requested.

| In the pipeline |
|-----------------|

For example, if the user runs:

```
mcat file.docx file.pdf -o inline
```

`mcat` will:
- Convert both `file.docx` and `file.pdf` into a single Markdown file
- Convert that Markdown into HTML
- Convert the HTML into an image
- Convert the image into an inline terminal image and print it

You can also start from the middle of the pipeline.  
For example:

```
mcat file.html -o image > image.png
```

This starts at an HTML file and directly converts it into a PNG image.
   
| Explanation of the blocks |  
|---------------------------|

* **`markdown`** - set when `-o md` or when the stdout isn't the terminal (piped)
* **`pretty terminal`** is markdown with ANSI formatting, and is the **default** for any non video / image file. (you can force it by adding the `-c` flag)
* **`html`** set when `-o html` -- only works for non image / video files
* **`static image`** set when `-o image` and gives an image
* **`interactive image`** set when `-o interactive` and launches an interactive view to zoom and pan the image in the terminal.
* **`inline display`** set when `-o inline` or `-i` and prints the content as image in the terminal

---
</details>


## 🏋️ Example Usage
```sh
#------------------------------------#
#  View a documents in the terminal  #
#------------------------------------#

mcat resume.pdf
mcat project.docx -t monokai                        # With a different theme
mcat "https://realpdfs.com/file.pdf"                # From a url
cat file.pptx | mcat                                # From stdin
mcat .                                              # Select files interactively

#-----------------# 
#  Convert files  #
#-----------------#

mcat archive.zip > README.md                        # Into Markdown
mcat src/main.rs src/lib.rs -o html > index.html    # Into HTML
mcat index.html -o image > page.png                 # Into image

#--------------------------#
#  View Images and Videos  #
#  in the terminal         #
#--------------------------#

mcat img.png                                        # Image
mcat video.mp4                                      # Video
mcat "https://giphy.com/gifs/..."                   # From a URL
mcat README.md -i                                   # Converts to image and then shows it
mcat ls                                             # ls command with images
mcat massive_image.png -o interactive               # zoom and pan the image interactively in the terminal

#--------------------------#
#  What I use it most for  #
#--------------------------#

mcat ls                                             # To find the image i was looking for
mcat . | scb                                        # Selects files, concat them, and copy to clipboard ~ for AI prompts
mcat index.html -o image > save.png                 # Render HTML into images
```

## 🛐 Dependencies
<details>
<summary><strong>Chromium (for rendering HTML to image)</strong></summary>

---
1. Available by default on most Windows machines via Microsoft Edge.
2. Also works with any installed Chrome, Edge, or Chromium.
3. You can install it manually via `mcat --fetch-chromium`
---
</details>

<details>
<summary><strong>FFmpeg (for videos)</strong></summary>

---
1. If it's already on your machine 🫠.
2. Otherwise, you can install it with `mcat --fetch-ffmpeg`
---
</details>

## ⚙️ Configuring
<details>
<summary><strong>Using Flags</strong></summary>

---
the main flags for configuring are:
* `--opts` for inline image printing
* `--ls-opts` for the ls command

run `mcat --help` for full detail, and other flags. 

---
</details>

<details>
<summary><strong>Using Environment Variables</strong></summary>

---
each variable mimicks its corresponding flag alternative.
* `MCAT_ENCODER`, Options: kitty,iterm,sixel,ascii. e.g. MCAT_ENCODER=kitty is the same as doing `--kitty`
* `MCAT_PAGER`, <str> the full command mcat will try to pipe into.
* `MCAT_THEME`, <str> same as the `--theme` flag
* `MCAT_INLINE_OPTS`, <str> same as the `--opts` flag
* `MCAT_LS_OPTS`, <str> same as the `--ls-opts` flag
* `MCAT_SILENT`, <bool> same as the `--silent` flag
* `MCAT_NO_LINENUMBERS`, <bool> same as the `--no-linenumbers` flag
---
</details>


## 🚧 Roadmap
- [ ] mcat.nvim: a neovim plugin to use mcat inside neovim

## 📎 License
MIT License
