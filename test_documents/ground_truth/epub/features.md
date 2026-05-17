# Reflowable EPUB 3 Conformance Test Document: 0100

## Status of this Document

This publication is currently considered \[UNDER DEVELOPMENT\] by the IDPF.

This publication is part of version X.X of the EPUB 3.0 Compliance Test Suite released on TBD.

Before using this publication to evaluate reading systems, testers are strongly encouraged to verify that they have the latest release by checking the current release version and date of the test suite at TBD[](http://idpf.org/)

This publication is one of several that currently comprise the EPUB 3 conformance test suite for reflowable content. The complete test suite includes all of the following publications:

- .

## About this Document

This document focuses on human-evaluated binary (pass/fail) tests in a reflowable context. Tests for fixed-layout content and other individual tests that require a dedicated epub file are available in additional sibling documents; refer to the test suite wiki (https:/[/github.com/mgylling/epu](https://github.com/mgylling/epub-testsuite/wiki/Overview)b-`testsuite/wiki/Overview) for additional information.`

## Conventions

The following conventions are used throughout the document:

## MathML

## \[REQUIRED\] &#9;&#9;&#9;&#9;mathml-010 Rendering

Tests whether MathML equation rendering is supported.

∫ − ∞ ∞ e − x 2 d x = π ∑ n = 1 ∞ 1 n 2 = π 2 6 x = − b ± b 2 − 4 a c 2 a

If the preceding equations are not presented as linear text (e.g., x=-b±b2-4ac2a), the test passes.

## \[OPTIONAL\] &#9;&#9;&#9;&#9;mathml-020 CSS Styling of the math element

Tests whether basic CSS styling of MathML is supported on the `math` element.

2 ⁡ x + y - z

The test passes if the equation has a yellow background and a dashed border.

If the reading system does not have a viewport, or does not support CSS styles, this test should be marked Not Supp`orted.`

## \[OPTIONAL\] mathml-021 CSS Styling of the mo element

Tests whether basic CSS styling of MathML is supported on the `mo` element.

2 ⁡ x + y - z

The test passes if the operators are enlarged relative to the other symbols and numbers.

If the reading system does not have a viewport, or does not support CSS styles, this test should be marked Not Supported.

## \[OPTIONAL\] mathml-022 CSS Styling of the mi element

Tests whether basic CSS styling of MathML is supported on the `mi` element.

2 ⁡ x + y - z

The test passes if the identifiers are bolded and blue.

If the reading system does not have a viewport, or does not support CSS styles, this test should be marked Not Supported.

## \[OPTIONAL\] mathml-023 CSS Styling of the mn element

Tests whether basic CSS styling of MathML is supported on the `mn` element.

2 ⁡ x + y - z

The test passes if the number 2 is italicized and blue.

If the reading system does not have a viewport, or does not support CSS styles, this test should be marked Not Supported.

## \[REQUIRED\] mathml-024Horizontal stretch, mover, munder, and mspace elements

Tests whether horizontal stretch, `mover`, `munder`, `mspace` elements are supported.

c = a ⏟ real + b ⁢ ⅈ ⏟ imaginary ⏞ complex number

The test passes if the rendering looks like .

## \[REQUIRED\] mathml-025Testing mtable with colspan and rowspan attributes, Hebrew and Script fonts

Tests whether `mtable` with `colspan` and `mspace` attributes (column and row spanning) are supported; uses Hebrew and Script alphabets.

cov ℒ ⟶ non 𝒦 ⟶ cof 𝒦 ⟶ cof ℒ ⟶ 2 ℵ 0 ↑ ↑ ↑ ↑ 𝔟 ⟶ 𝔡 ↑ ↑ ℵ 1 ⟶ add ℒ ⟶ add 𝒦 ⟶ cov 𝒦 ⟶ non ℒ

The test passes if the rendering looks like [Cichoń's Diagram](http://en.wikipedia.org/wiki/Cicho%C5%84's_diagram): .

## \[REQUIRED\] mathml-026BiDi, RTL and Arabic alphabets

Tests whether right-to-left and Arabic alphabets are supported.

د ⁡ ( س ) = { ∑ ٮ = 1 ص ⁡ س ٮ إذاكان س \> 0 ∫ 1 ص ⁡ س ٮ ⁢ ء ⁡ س إذاكان س ∈ م طا ⁡ π غيرذلك ( مع π ≃ 3,141 )

The test passes if the rendering looks like the following image:

## \[REQUIRED\] mathml-027Elementary math: long division notation

Tests whether `mlongdiv` elements (from elementary math) are supported.

3 435.3 1306 12 10 9 16 15 1.0 9 1

The test passes if the rendering looks like the following image: .

### epub:switch

#### \[REQUIRED\] switch-010 Support

Tests whether the [epub:switch](http://idpf.org/epub/30/spec/epub30-contentdocs.html#sec-xhtml-content-switch) element is supported.

FAIL

PASS

If only the word "PASS" is rendered before this paragraph, the test passes. If both "PASS" and "FAIL" are rendered, or neither "PASS" nor "FAIL" is rendered, the test fails.

#### \[OPTIONAL\] &#9;&#9;&#9;switch-020 &#9;&#9;&#9;MathML Embedding

Tests whether the MathML namespace is recognized when used in an [epub:case](http://idpf.org/epub/30/spec/epub30-contentdocs.html#sec-xhtml-epub-case) element.

2 ⁡ x + y - z

FAIL

If a MathML equation is rendered before this paragraph, the test passes.

If test `switch-010` did not pass, this test should be marked `Not Supported`.
