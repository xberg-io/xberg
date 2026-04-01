# Pandoc Test Suite

John MacFarlane

## Headers

### Level 2 with an [embedded link](/url)

#### Level 3 with *emphasis*

##### Level 4

## Level 1

### Level 2 with *emphasis*

#### Level 3

with no blank line

### Level 2

with no blank line

## Paragraphs

Here's a regular paragraph.

In Markdown 1.0.0 and earlier. Version 8.

Here's one with a bullet. \* criminey.

There should be a hard line break\
here.

## Block Quotes

E-mail style:

> This is a block quote. It is pretty short.

This should not be a block quote: 2 > 1.

And a following paragraph.

## Special Characters

Here is some unicode:

- I hat: Î
- o umlaut: ö
- section: §
- set membership: ∈
- copyright: ©

AT&T has an ampersand in their name.

AT&T is another way to write it.

This & that.

4 < 5.

6 > 5.

Backslash: \\

Backtick: \`

Asterisk: \*

Underscore: \_

## Code Examples

``` python
def example():
    return True
```

## Tables

| Header 1 | Header 2 |
|----------|----------|
| Data 1   | Data 2   |
| Data 3   | Data 4   |

## Links and emphasis

An *[emphasized link](/url)*.

This is code: `>=`, `$`, `\`, `\$`, `<html>`.

~~This is *strikeout*.~~

This is *emphasized*, and so *is this*.

This is **strong**, and so **is this**.

Here's a [link with an ampersand in the URL](http://example.com/?foo=1&bar=2).

Here's a link with an ampersand in the link text: [AT&T](http://att.com/).
