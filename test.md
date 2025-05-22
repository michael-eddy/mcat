## FrontMatter
```yaml
---
title: "Example Document"
author: "Claude"
date: 2025-05-22
tags: [markdown, examples]
---
```

---

## BlockQuote
> This is a simple block quote.
> It can span multiple lines and paragraphs.
> > This is a nested block quote.

---

## List (Unordered)
- First item
- Second item
  - Nested item
  - Another nested item
- Third item

## List (Ordered)
1. First numbered item
2. Second numbered item
   1. Nested numbered item
   2. Another nested item
3. Third numbered item

---

## DescriptionList
<dl>
<dt>Term 1</dt>
<dd>Description for term 1</dd>
<dt>Term 2</dt>
<dd>Description for term 2</dd>
</dl>

---

## DescriptionTerm
*Each `<dt>` element above represents a NodeValue::DescriptionTerm*

---

## DescriptionDetails
*Each `<dd>` element above represents a NodeValue::DescriptionDetails*

---

## CodeBlock
```python
def hello_world():
    print("Hello, World!")
    return True
```

```javascript
function greet(name) {
    console.log(`Hello, ${name}!`);
}
```

    # Indented code block
    echo "This is also a code block"

---

## HtmlBlock
<div class="example">
    <p>This is an HTML block.</p>
    <strong>It can contain any HTML elements.</strong>
</div>

<table>
    <tr>
        <td>HTML Table</td>
        <td>In a block</td>
    </tr>
</table>

---

## Paragraph
This is a regular paragraph. It contains normal text that flows naturally and can span multiple lines when it gets long enough to wrap.

This is another paragraph, separated by a blank line from the previous one.

---

## Heading
# Heading Level 1
## Heading Level 2
### Heading Level 3
#### Heading Level 4
##### Heading Level 5
###### Heading Level 6

Alternative Heading 1
=====================

Alternative Heading 2
---------------------

---

## ThematicBreak
Above and below this text are thematic breaks (horizontal rules):

---

***

___

---

## FootnoteDefinition
Here's a sentence with a footnote[^1].

And another with a different footnote[^note2].

[^1]: This is the first footnote definition.

[^note2]: This is the second footnote definition with more details.

---

## Table
| Column 1 | Column 2 | Column 3 |
|----------|----------|----------|
| Row 1, Col 1 | Row 1, Col 2 | Row 1, Col 3 |
| Row 2, Col 1 | Row 2, Col 2 | Row 2, Col 3 |

| Left Aligned | Center Aligned | Right Aligned |
|:-------------|:--------------:|--------------:|
| Left | Center | Right |
| Content | Content | Content |

---

## TableRow
*Each row in the tables above represents a NodeValue::TableRow*

---

## TableCell
*Each cell in the tables above represents a NodeValue::TableCell*

---

## Text
*Most of the plain text content in this document represents NodeValue::Text*

---

## TaskItem
- [x] Completed task
- [ ] Incomplete task
- [x] Another completed task
  - [ ] Nested incomplete task
  - [x] Nested completed task

---

## SoftBreak
This line has a soft break
right here where it continues.

---

## LineBreak
This line has a hard line break  
Right here on the next line.

You can also use a backslash\
for a line break.

---

## Code
Here's some `inline code` in a sentence.

You can also use `code with backticks` or even ``code with `backticks` inside``.

---

## HtmlInline
This paragraph contains <strong>inline HTML</strong> and <em>emphasis</em>.

You can also use <code>inline code tags</code> and <span style="color: red;">styled spans</span>.

---

## Raw
Raw content typically appears in specialized contexts and may not render visibly.

---

## Emph
This text has *emphasis* using asterisks.

This text has _emphasis_ using underscores.

---

## Strong
This text has **strong importance** using asterisks.

This text has __strong importance__ using underscores.

---

## Strikethrough
This text has ~~strikethrough~~ formatting.

---

## Superscript
E = mc^2^

The 1^st^ of January.

---

## Link
Here's a [link to Google](https://www.google.com).

Here's a [link with a title](https://www.example.com "Example Website").

This is a reference-style link to [Google][1].

[1]: https://www.google.com

---

## Image
![Alt text for an image](https://via.placeholder.com/150x100)

![Image with title](https://via.placeholder.com/200x150 "Placeholder Image")

Reference-style image: ![Alt text][image1]

[image1]: https://via.placeholder.com/100x100

---

## Image with link

[![Rust Logo](https://www.rust-lang.org/logos/rust-logo-512x512.png)](https://www.rust-lang.org/)

---

## FootnoteReference
*The `[^1]` and `[^note2]` in the FootnoteDefinition section above represent NodeValue::FootnoteReference*

---

## Math
Inline math: $E = mc^2$

Block math:
$$
\sum_{i=1}^{n} x_i = x_1 + x_2 + \cdots + x_n
$$

---

## MultilineBlockQuote
> This is a multiline block quote
that spans several lines
and maintains its formatting
It can contain multiple paragraphs
and other elements like:

- Lists
- **Bold text**
- `Code`

---

## Escaped
Here are some escaped characters: \* \_ \` \# \[ \]

You can escape \*asterisks\* and \_underscores\_ to prevent formatting.

---

## WikiLink
[[Internal Link]]

[[Link with Display Text|Display Text]]

[[Category:Examples]]

---

## Underline
<u>This text is underlined</u> using HTML.

---

## Subscript
H~2~O is the chemical formula for water.

CO~2~ is carbon dioxide.

---

## SpoileredText
||This text is spoilered|| and hidden by default.

You can reveal ||spoiler content|| by clicking on it.

---

## EscapedTag
\<this-is-not-html\>

\<script\>alert('xss')\</script\>

---

## Alert
> [!NOTE]
> This is a note alert with important information.

> [!TIP]
> This is a tip alert with helpful advice.

> [!IMPORTANT]
> This is an important alert that draws attention.

> [!WARNING]
> This is a warning alert about potential issues.

> [!CAUTION]
> This is a caution alert for dangerous situations.

---

## Summary

This document demonstrates examples for all NodeValue types in markdown parsing libraries. Each section shows how the different node types appear in actual markdown content, covering both CommonMark standard features and GitHub Flavored Markdown extensions.
