# Comprehensive Comrak/GFM Markdown Reference

## Basic Formatting

**Bold text** and __also bold text__

*Italic text* and _also italic text_

***Bold and italic*** or ___triple emphasis___

~~Strikethrough~~

> Blockquotes
> > Nested blockquotes
> > > Deeper nesting

## Headings

# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6

Alternative Heading 1
=====================

Alternative Heading 2
---------------------

## Links

[Basic link](https://example.com)

[Link with title](https://example.com "Link title")

[Reference link][ref]

[ref]: https://example.com "Reference Link"

<https://example.com> (Autolinks)

## Images

![Alt text for image](https://example.com/image.jpg)

![Alt text with title](https://example.com/image.jpg "Image title")

[![Image with link](https://example.com/image.jpg)](https://example.com)

## Lists

### Unordered Lists

* Item 1
* Item 2
  * Nested item
    * Deeper nested item
* Item 3

- Alternative bullet
+ Another alternative

### Ordered Lists

1. First item
2. Second item
   1. Nested ordered item
   2. Another nested item
3. Third item

### Task Lists (GFM specific)

- [x] Completed task
- [ ] Incomplete task
- [ ] \(Escaped parentheses in task)

### Definition Lists

Term
: Definition
: Another definition

## Code

Inline `code` with backticks

```
Code block without syntax highlighting
```

```javascript
// Code block with syntax highlighting
function example() {
  console.log("Hello, world!");
}
```

    Indented code block
    (4 spaces or 1 tab)

## Tables (GFM specific)

| Header 1 | Header 2 | Header 3 |
|----------|:--------:|---------:|
| Default  | Center   | Right    |
| aligned  | aligned  | aligned  |
| cells    | cells    | cells    |

| Simple | Table |
| ------ | ----- |
| No     | frills|

## Horizontal Rules

---

***

___

## Footnotes

Here's a sentence with a footnote[^1].

[^1]: This is the footnote content.

## Superscript and Subscript Extensions

H~2~O (subscript)

X^2^ (superscript)

## Escaping Characters

\* Escaped asterisk \*

\\ Escaped backslash

\` Escaped backtick

## HTML (Supported in GFM/Comrak)

<details>
<summary>Expandable section</summary>
Content inside expandable section
</details>

<div align="center">
Centered content
</div>

## URL Auto-linking

https://example.com

www.example.com

## Emoji Shortcodes (GFM specific)

:smile: :heart: :thumbsup:

## Line Breaks

Line with two spaces at end  
Next line

Line with backslash at end\
Next line

## Comments

[//]: # (This is a comment that won't appear in the rendered output)

<!--
HTML style comment
-->

## Extensions

### Highlighted Text

==Highlighted text== (if enabled in the parser)

### Admonitions/Callouts

> [!Note]
> This is a note admonition block in GitHub style

> [!Warning]
> This is a warning admonition block

> [!Important]
> This highlights important information 

> [!Tip]
> This is a helpful tip

> [!Caution]
> This indicates a dangerous or error-prone step

> **Note**
> This is an alternative note style

### Fancy Lists (with attributes)

1. First item
   {: .custom-class}
2. Second item
   {: #custom-id}

## Math (if MathJax/KaTeX support is enabled)

Inline: $E = mc^2$

Display: 
$$
\frac{d}{dx}(e^x) = e^x
$$
