=========================
reStructuredText Document
=========================

:Author: Emily Chen
:Date: 2025-09-27
:Version: 1.0
:Organization: Testing Department
:Abstract: This reStructuredText document tests pandoc extraction capabilities
           with RST-specific features and metadata.

.. contents:: Table of Contents
   :depth: 2

Introduction
============

reStructuredText is a plaintext markup syntax used primarily in Python documentation.
This document tests various RST features.

Text Formatting
===============

Basic Formatting
----------------

This paragraph contains *emphasized text*, **strong text**, and ``inline code``.

You can also use:

* Bullet lists
* With multiple items

  - And nested items
  - Like this

1. Numbered lists
2. Also work well
#. Auto-numbering is supported

Code Blocks
===========

Python Example
--------------

.. code-block:: python

    def process_document(doc):
        """Process an RST document."""
        metadata = extract_metadata(doc)
        content = parse_content(doc)
        return {"metadata": metadata, "content": content}

Shell Example
-------------

.. code-block:: bash

    $ pandoc input.rst -o output.md
    $ echo "Conversion complete"

Directives and Roles
====================

.. note::
   This is a note directive.

.. warning::
   This is a warning directive.

.. tip::
   RST supports various admonitions.

Tables
======

Simple Table
------------

=====  =====  ======
Col1   Col2   Col3
=====  =====  ======
A      B      C
D      E      F
=====  =====  ======

Grid Table
----------

+-------+-------+-------+
| Head1 | Head2 | Head3 |
+=======+=======+=======+
| Cell1 | Cell2 | Cell3 |
+-------+-------+-------+
| Cell4 | Cell5 | Cell6 |
+-------+-------+-------+

Citations and References
========================

According to [Smith2024]_, RST is widely used in technical documentation.

.. [Smith2024] Smith, J. (2024). *Technical Documentation with RST*. Doc Press.

Links and References
====================

External link: `Python Documentation <https://docs.python.org>`_

Internal reference: See `Introduction`_ section.

Conclusion
==========

This document demonstrates RST features for comprehensive pandoc testing.
