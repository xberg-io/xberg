# Pandoc Test Suite

## Subtitle

Authors  
John MacFarlane; Anonymous

Date  
July 17, 2006

Revision  
3

### Level one header

This is a set of tests for pandoc. Most of them are adapted from John Gruber's markdown test suite.

#### Level two header

##### Level three

###### Level four with *emphasis*

####### Level five

### Paragraphs

Here's a regular paragraph.

In Markdown 1.0.0 and earlier. Version 8. This line turns into a list item. Because a hard-wrapped line in the middle of a paragraph looked like a list item.

Here's one with a bullet. \* criminey.

Horizontal rule:

------------------------------------------------------------------------

Another:

------------------------------------------------------------------------

### Block Quotes

Here's a block quote:

> This is a block quote. It is pretty short.

Here's another, differently indented:

> This is a block quote. It's indented with a tab.
>
> Code in a block quote:
>
>     sub status {
>         print "working";
>     }
>
> List in a block quote:
>
> 1.  item one
> 2.  item two
>
> Nested block quotes:
>
> > nested
> >
> > > nested

### Code Blocks

Code:

    ---- (should be four hyphens)

    sub status {
        print "working";
    }

    this code block is indented by one tab

And:

    this block is indented by two tabs

    These should not be escaped:  $ \\ \> \[ \{

And:

``` python
def my_function(x):
    return x + 1
```

If we use the highlight directive, we can specify a default language for literate blocks.

``` haskell
-- this code is in haskell
data Tree = Leaf | Node Tree Tree
```

``` haskell
-- this code is in haskell too
data Nat = Zero | Succ Nat
```

``` javascript
-- this code is in javascript
let f = (x, y) => x + y
```

### Lists

#### Unordered

Asterisks tight:

- asterisk 1
- asterisk 2
- asterisk 3

Asterisks loose:

- asterisk 1
- asterisk 2
- asterisk 3

Pluses tight:

- Plus 1
- Plus 2
- Plus 3

Pluses loose:

- Plus 1
- Plus 2
- Plus 3

Minuses tight:

- Minus 1
- Minus 2
- Minus 3

Minuses loose:

- Minus 1
- Minus 2
- Minus 3

#### Ordered

Tight:

1.  First
2.  Second
3.  Third

and:

1.  One
2.  Two
3.  Three

Loose using tabs:

1.  First
2.  Second
3.  Third

and using spaces:

1.  One
2.  Two
3.  Three

Multiple paragraphs:

1.  Item 1, graf one.

    Item 1. graf two. The quick brown fox jumped over the lazy dog's back.

2.  Item 2.

3.  Item 3.

Nested:

- Tab
  - Tab
    - Tab

Here's another:

1.  First

2.  Second:

    > - Fee
    > - Fie
    > - Foe

3.  Third

#### Fancy list markers

2)  begins with 2

3)  and now 3

    with a continuation

    4.  sublist with roman numerals, starting with 4
    5.  more items
        1)  a subsublist
        2)  a subsublist

Nesting:

1.  Upper Alpha
    1.  Upper Roman.
        6)  Decimal start with 6
            3)  Lower alpha with paren

Autonumbering:

1.  Autonumber.
2.  More.
    1.  Nested.

Autonumbering with explicit start:

4)  item 1
5)  item 2

#### Definition

term 1  
Definition 1.

term 2  
Definition 2, paragraph 1.

Definition 2, paragraph 2.

term with *emphasis*  
Definition 3.

### Field Lists

> address  
> 61 Main St.
>
> city  
> *Nowhere*, MA, USA
>
> phone  
> 123-4567

address  
61 Main St.

city  
*Nowhere*, MA, USA

phone  
123-4567

### HTML Blocks

Simple block on one line:

foo

Now, nested:

foo

### LaTeX Block

### Inline Markup

This is *emphasized*. This is **strong**.

This is code: `>`, `$`, `\`, `$`, ``.

This is~subscripted~ and this is ^superscripted^.

### Special Characters

Here is some unicode:

- I hat: Î
- o umlaut: ö
- section: §
- set membership: ∈
- copyright: ©

AT&T has an ampersand in their name.

This & that.

4 \< 5.

6 \> 5.

Backslash: \\

Backtick: \`

Asterisk: \*

Underscore: \_

Left brace: {

Right brace: }

Left bracket: \[

Right bracket: \]

Left paren: (

Right paren: )

Greater-than: \>

Hash: \#

Period: .

Bang: !

Plus: +

Minus: -

### Links

Explicit: a [URL](/url/).

Explicit with no label: [foo](foo).

Two anonymous links: [the first](/url1/) and [the second](/url2/)

Reference links: [link1](/url1/) and [link2](/url2/) and [link1](/url1/) again.

Another [style of reference link](/url1/).

Here's a [link with an ampersand in the URL](http://example.com/?foo=1&bar=2).

Here's a link with an amersand in the link text: [AT&T](/url/).

Autolinks: <http://example.com/?foo=1&bar=2> and <nobody@nowhere.net>.

But not here:

    http://example.com/

### Images

From "Voyage dans la Lune" by Georges Melies (1902):

![image](lalune.jpg)

![Voyage dans la Lune](lalune.jpg)

Here is a movie ![movie](movie.jpg) icon.

And an [![A movie](movie.jpg)](/url).

### Comments

First paragraph

Another paragraph

A third paragraph

### Line blocks

But can a bee be said to be\
    or not to be an entire bee,\
        when half the bee is not a bee,\
            due to some ancient injury?\
\
Continuation line\
  and another

### Simple Tables

| col 1 | col 2 | col 3 |
|-------|-------|-------|
| r1 a  | b     | c     |
| r2 d  | e     | f     |

Headless

|      |     |     |
|------|-----|-----|
| r1 a | b   | c   |
| r2 d | e   | f   |

### Grid Tables

| col 1       | col 2 | col 3 |
|-------------|-------|-------|
| r1 a r1 bis | b b 2 | c c 2 |
| r2 d        | e     | f     |

Headless

|             |       |       |
|-------------|-------|-------|
| r1 a r1 bis | b b 2 | c c 2 |
| r2 d        | e     | f     |

Spaces at ends of lines

|             |       |       |
|-------------|-------|-------|
| r1 a r1 bis | b b 2 | c c 2 |
| r2 d        | e     | f     |

Multiple blocks in a cell

|             |               |           |
|-------------|---------------|-----------|
| r1 a r1 bis | b, b 2, b 2   | c c 2 c 2 |

Table with cells spanning multiple rows or columns:

| Property               |      | Earth    |
|------------------------|------|----------|
| Temperature 1961-1990  | min  | -89.2 °C |
|                        | mean | 14 °C    |
|                        | min  | 56.7 °C  |

Table with complex header:

| Location   | min   | mean | max  |
|------------|-------|------|------|
| Antarctica | -89.2 | N/A  | 19.8 |
| Earth      | -89.2 | 14   | 56.7 |

### Footnotes

[^1]

[^2]

[^3]

[^4]

Not in note.

### Math

Some inline math $`E=mc^2`$. Now some display math:

``` math
E=mc^2
```

``` math
E = mc^2
```

``` math
E = mc^2
```
``` math
\alpha = \beta
```

``` math
\begin{aligned}
E &= mc^2\\
F &= \pi E
\end{aligned}
```
``` math
F &= \gamma \alpha^2
```

All done.

### Default-Role

Try changing the default role to a few different things.

#### Doesn't Break Title Parsing

Inline math: $`E=mc^2`$ or $`E=mc^2`$ or $`E=mc^2`$. Other roles: ^super^, ~sub~.

``` math
\alpha = beta
```
``` math
E = mc^2
```

Some ^of^ these ^words^ are in ^superscript^.

Reset default-role to the default default.

And now some-invalid-string-3231231 is nonsense.

And now with **inline** HTML.

And some inline haskell `fmap id [1,2..10]`.

Indirect python role `[x*x for x in [1,2,3,4,5]]`.

Different indirect C `int x = 15;`.

#### Literal symbols

2\*2 = 4\*1

[^1]: Note with one line.

[^2]: Note with continuation line.

[^3]: Note with

    continuation block.

[^4]: Note with continuation line

    and a second para.
