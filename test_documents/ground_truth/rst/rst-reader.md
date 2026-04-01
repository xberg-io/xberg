## Pandoc Test Suite

### Subtitle

**Authors**: John MacFarlane; Anonymous

**Date**: July 17, 2006

**Revision**: 3

#### Level one header

This is a set of tests for pandoc. Most of them are adapted from

John Gruber's markdown test suite.

##### Level two header

###### Level three

####### Level four with \*emphasis\*

Level five

''''''''''

#### Paragraphs

Here's a regular paragraph.

In Markdown 1.0.0 and earlier. Version

1. This line turns into a list item.

Because a hard-wrapped line in the

middle of a paragraph looked like a

list item.

Here's one with a bullet.

- criminey.

Horizontal rule:

Another:

#### Block Quotes

Here's a block quote:

This is a block quote.

It is pretty short.

Here's another, differently indented:

This is a block quote.

It's indented with a tab.

Code in a block quote::

sub status {

print "working";

}

List in a block quote:

1. item one
2. item two

Nested block quotes:

nested

nested

#### Code Blocks

Code:

: 

\---- (should be four hyphens)

sub status {

print "working";

}

: 

this code block is indented by one tab

And::

this block is indented by two tabs

These should not be escaped: \\$ \\\\ \\\> \\\[ \\{

And:

```python
def my_function(x):
    return x + 1
```

If we use the highlight directive, we can specify a default language

for literate blocks.

: 

\-- this code is in haskell

data Tree = Leaf | Node Tree Tree

: 

\-- this code is in haskell too

data Nat = Zero | Succ Nat

: 

\-- this code is in javascript

let f = (x, y) =\> x + y

#### Lists

##### Unordered

Asterisks tight:

\*&#9;asterisk 1

\*&#9;asterisk 2

\*&#9;asterisk 3

Asterisks loose:

\*&#9;asterisk 1

\*&#9;asterisk 2

\*&#9;asterisk 3

Pluses tight:

\+&#9;Plus 1

\+&#9;Plus 2

\+&#9;Plus 3

Pluses loose:

\+&#9;Plus 1

\+&#9;Plus 2

\+&#9;Plus 3

Minuses tight:

\-&#9;Minus 1

\-&#9;Minus 2

\-&#9;Minus 3

Minuses loose:

\-&#9;Minus 1

\-&#9;Minus 2

\-&#9;Minus 3

##### Ordered

Tight:

1\.&#9;First

2\.&#9;Second

3\.&#9;Third

and:

1. One
2. Two
3. Three

Loose using tabs:

1\.&#9;First

2\.&#9;Second

3\.&#9;Third

and using spaces:

1. One

<!-- end list -->

1. Two

<!-- end list -->

1. Three

Multiple paragraphs:

1\.&#9;Item 1, graf one.

Item 1. graf two. The quick brown fox jumped over the lazy dog's

back.

2\.&#9;Item 2.

3\.&#9;Item 3.

Nested:

\*&#9;Tab

\*&#9;Tab

\*&#9;Tab

Here's another:

1. First

<!-- end list -->

1. Second:

<!-- end list -->

- Fee
- Fie
- Foe

<!-- end list -->

1. Third

##### Fancy list markers

(2) begins with 2

(3) and now 3

with a continuation

iv. sublist with roman numerals, starting with 4

v. more items

(A) a subsublist

(B) a subsublist

Nesting:

A. Upper Alpha

I. Upper Roman.

(6) Decimal start with 6

c) Lower alpha with paren

Autonumbering:

1. Autonumber.
2.  More.

<!-- end list -->

1. Nested.

Autonumbering with explicit start:

(d) item 1

(\#) item 2

##### Definition

term 1

Definition 1.

term 2

Definition 2, paragraph 1.

Definition 2, paragraph 2.

term with *emphasis*

Definition 3.

#### Field Lists

**address**: 61 Main St.

**city**: \*Nowhere\*, MA,

**phone**: 123-4567

**address**: 61 Main St.

**city**: \*Nowhere\*, MA,

**phone**: 

123-4567

#### HTML Blocks

Simple block on one line:

Now, nested:

\<div\>

\<div\>

foo

\</div\>

\</div\>

\</div\>

#### LaTeX Block

#### Inline Markup

This is *emphasized*. This is **strong**.

This is code: `>`, `$`, `\`, `\$`, `<html>`.

This is\\ :sub:`subscripted` and this is :sup:`superscripted`\\ .

#### Special Characters

Here is some unicode:

- I hat: Î
- o umlaut: ö
- section: §
- set membership: ∈
- copyright: ©

AT\&T has an ampersand in their name.

This & that.

4 \< 5.

6 \> 5.

Backslash: \\\\

Backtick: \\\`

Asterisk: \\\*

Underscore: \\_

Left brace: \\{

Right brace: \\}

Left bracket: \\\[

Right bracket: \\\]

Left paren: \\(

Right paren: \\)

Greater-than: \\\>

Hash: \\\#

Period: \\.

Bang: \\\!

Plus: \\+

Minus: \\-

#### Links

Explicit: a [URL](/url/).

Explicit with no label: .

Two anonymous links: `the first`_ and `the second`_

__ /url1/

__ /url2/

Reference links: `link1` and `link2` and link1_ again.

[link1](/url1/)

[\`link2\`](/url2/)

Another [style of reference link](link1_).

Here's a `link with an ampersand in the URL`.

Here's a link with an amersand in the link text: [AT\&T](/url/).

[link with an ampersand in the URL](http://example.com/?foo=1&bar=2)

Autolinks: http://example.com/?foo=1\&bar=2 and nobody@nowhere.net.

But not here::

http://example.com/

#### Images

From "Voyage dans la Lune" by Georges Melies (1902):

\[image: lalune.jpg\]

\[image: Voyage dans la Lune\]

Here is a movie |movie| icon.

And an |image with a link|.

#### Comments

First paragraph

Another paragraph

A third paragraph

#### Line blocks

| But can a bee be said to be

| or not to be an entire bee,

| when half the bee is not a bee,

| due to some ancient injury?

|

| Continuation

line

| and

another

#### Simple Tables

| col 1 | col 2 | col 3 |
| --- | --- | --- |

r1 a b c

r2 d e f

Headless

| r1 a | b | c |
| --- | --- | --- |
| r2 d | e | f |

#### Grid Tables

| col 1 | col 2 | col 3 |
| --- | --- | --- |
| r1 a | b | c |
| r1 bis | b 2 | c 2 |
| r2 d | e | f |

Headless

| r1 a | b | c |
| --- | --- | --- |
| r1 bis | b 2 | c 2 |
| r2 d | e | f |

Spaces at ends of lines

| r1 a | b | c |
| --- | --- | --- |
| r1 bis | b 2 | c 2 |
| r2 d | e | f |

Multiple blocks in a cell

| r1 a | - b | c |
| --- | --- | --- |
| - b 2 | c 2 |  |
| r1 bis | - b 2 | c 2 |

Table with cells spanning multiple rows or columns:

| Property | Earth |  |
| --- | --- | --- |
| min | -89.2 °C |  |
| Temperature +-------+----------+ |  |  |
| 1961-1990 | mean | 14 °C |
| min | 56.7 °C |  |

Table with complex header:

| Location | Temperature 1961-1990 |  |  |
| --- | --- | --- | --- |
| in degree Celsius |  |  |  |
| min | mean | max |  |
| Antarctica | -89.2 | N/A | 19.8 |
| Earth | -89.2 | 14 | 56.7 |

#### Footnotes

\[1\]_[^1]

\[\#\]_[^2]

\[\#\]_[^2]

\[\*\]_

continuation line.

continuation block.

and a second para.

Not in note.

#### Math

Some inline math :math:`E=mc^2`\\ . Now some

display math:

$$E=mc^2$$

$$E = mc^2$$

$$E = mc^2
\alpha = \beta$$

$$:label: hithere
:nowrap:
E &= mc^2\\
F &= \pi E
F &= \gamma \alpha^2$$

All done.

#### Default-Role

Try changing the default role to a few different things.

##### Doesn't Break Title Parsing

Inline math: `E=mc^2` or :math:`E=mc^2` or `E=mc^2`:math:.

Other roles: :sup:`super`, `sub`:sub:.

$$\alpha = beta
E = mc^2$$

Some `of` these :sup:`words` are in `superscript`:sup:.

Reset default-role to the default default.

And now `some-invalid-string-3231231` is nonsense.

And now with :html:`<b>inline</b> <span id="test">HTML</span>`.

And some inline haskell :haskell:`fmap id [1,2..10]`.

Indirect python role :py:`[x*x for x in [1,2,3,4,5]]`.

Different indirect C :c:`int x = 15;`.

##### Literal symbols

2*2 = 4*1

[^1]:
    Note with one line.

[^2]:
    Note with
