# PubTables-1M: Towards comprehensive table extraction from unstructured
documents

Brandon Smock
Rohith Pesala
Robin Abraham
Microsoft
Redmond, WA
brsmock, ropesala, robin.abraham@microsoft.com

Abstract

Recently, significant progress has been made applying
machine learning to the problem of table structure inference
and extraction from unstructured documents. However, one
of the greatest challenges remains the creation of datasets
with complete, unambiguous ground truth at scale. To ad-
dress this, we develop a new, more comprehensive dataset
for table extraction, called PubTables-1M. PubTables-1M
contains nearly one million tables from scientific articles,
supports multiple input modalities, and contains detailed
header and location information for table structures, making
it useful for a wide variety of modeling approaches. It also
addresses a significant source of ground truth inconsistency
observed in prior datasets called oversegmentation, using a
novel canonicalization procedure. We demonstrate that these
improvements lead to a significant increase in training per-
formance and a more reliable estimate of model performance
at evaluation for table structure recognition. Further, we
show that transformer-based object detection models trained
on PubTables-1M produce excellent results for all three tasks
of detection, structure recognition, and functional analysis
without the need for any special customization for these tasks.
Data and code will be released at https://github.
com/microsoft/table-transformer.

1. Introduction

A table is a compact, structured representation for storing
data and communicating it in documents and other manners
of presentation. In its presented form, however, a table, such
as the one in Fig. 1, may not and often does not explicitly
represent its logical structure. This is an important problem
as a significant amount of data is communicated through doc-
uments, but without structure information this data cannot
be used in further applications.

The problem of inferring a table's structure from its pre-
sentation and converting it to a structured form is known as
<!-- image -->
Figure 1. An example of a presentation table whose underlying
structure must be inferred, either manually or by automated sys-
tems.

table extraction (TE). TE entails three subtasks [6], which we
illustrate in Fig. 2: table detection (TD), which locates the
table; table structure recognition (TSR), which recognizes
the structure of a table in terms of rows, columns, and cells;
and functional analysis (FA), which recognizes the keys and
values of the table. TE is challenging for automated sys-
tems [9, 12, 17,23] due to the wide variety of formats, styles,
and structures found in presented tables.

Recently, there has been a shift in the research litera-
ture from traditional rule-based methods [4, 11, 18] for TE to
data-driven methods based on deep learning (DL) [14,17,22].
The primary advantage of DL methods is that they can learn
to be more robust to the wide variety of table presentation
formats. However, manually annotating tables for TSR is a
difficult and time-consuming process [7]. To overcome this,
researchers have turned recently to crowd-sourcing to con-
struct larger datasets [9,22,23]. These datasets are assembled
from tables appearing in documents created by thousands
of authors, where an annotation for each table's structure
and content is available in a markup format such as HTML,
XML, or LaTeX.

While crowd-sourcing solves the problem of dataset size,
repurposing annotations originally unintended for TE and
automatically converting these to ground truth presents its
own set of challenges with respect to completeness, consis-
tency, and quality. This includes not only what information
is present but how explicitly this information is represented.

<!-- image -->
Figure 2. Illustration of the three subtasks of table extraction addressed by the PubTables-1M dataset.

For instance, markup annotations do not encode spatial coor-
dinates for cells, and they only encode logical relationships
implicitly through cues such as layout [20]. Not only does
this lack of explicit information limit the range of supervised
modeling approaches, it also limits the quality control that
can be done to verify the annotations' correctness.

Another significant challenge for the use of crowd-
sourced annotations is that these structure annotations en-
coded in markup often exhibit an issue we refer to as over-
segmentation. Oversegmentation occurs in a structure anno-
tation when a spanning cell in a header is split into multiple
grid cells. We give examples of this in Fig. 3. Oversegmenta-
tion in markup usually has no effect on how a table appears
due to borders between cells being invisible, leaving a pre-
sentation table's implicit logical structure and interpretation
unaffected. However, oversegmentation can lead to signif-
icant issues when used as ground truth for model training
and evaluation.

The first issue is that an oversegmented annotation contra-
dicts the logical interpretation of the table that its presenta-
tion is meant to suggest. For instance, oversegmenting a cell
annotation may indicate that its text applies to only one row
when its presentation form suggests its text is meant to apply
to several rows, as in the cell in column 1, row 3 in Fig. 3.
This is problematic for use as ground truth to teach a ma-
chine learning model to correctly interpret a table's structure.
Even if oversegmented annotations were considered a valid
intepretation of a table's structure, allowing them would and
does lead to ambiguous and inconsistent ground truth, due to
there then being multiple possible valid interpretations for a
table's structure, such as in Fig. 3. This violates the standard
modeling assumption that there is exactly one correct ground
truth annotation for each table. Thus, datasets that contain
oversegmented annotations in them lead to inconsistent, con-
tradictory feedback during training and an underestimate of
true performance during evaluation.

To address these and other challenges, we develop a new
large-scale dataset for table extraction called PubTables-1M.
PubTables-1M contains nearly one million tables from scien-
tific articles in the PubMed Central Open Access¹ (PMCOA)
database. Among our contributions:

- PubTables-1M is nearly twice as large as the current
largest comparable dataset and addresses all three tasks
of table detection (TD), table structure recognition
(TSR), and functional analysis (FA).
- Compared to prior datasets, PubTables-1M contains
richer annotation information, including annotations
for projected row headers and bounding boxes for all
rows, columns, and cells, including blank cells. It also
includes annotations on their original source documents,
which supports multiple input modalities and enables a
wide range of potential model architectures.
- We introduce a novel canonicalization procedure that
corrects oversegmentation and whose goal is to ensure
each table has a unique, unambiguous structure inter-
pretation.
- To reduce additional sources of error, we implement
several quality verification and control steps and pro-
vide measurable guarantees about the quality of the
ground truth.
- We show that data improvements alone lead to a sig-
nificant increase in performance for TSR models, due
both to improved training and a more reliable estimate
of performance at evaluation.

¹https://www.ncbi.nlm.nih.gov/pmc/tools/openftlist/

<!-- image -->
Figure 3. In the above example, the structure annotation on the left is oversegmented, creating extra blank cells in the headers. The canonical
structure annotation on the right merges these cells and captures its true logical structure.

- Finally, we apply the Detection Transformer (DETR)
[2] for the first time to the tasks of TD, TSR, and FA,
and demonstrate how with PubTables-1M all three tasks
can be addressed with a transformer-based object detec-
tion framework without any special customization for
these tasks.

2. Related Work

Structure recognition datasets The first dataset to ad-
dress all three table extraction tasks was the ICDAR-2013
dataset [6]. It remains popular for benchmarking TSR mod-
els due to its quality and relative completeness compared to
other datasets. However, as a source of training data for table
extraction models it is limited, containing only 248 tables
for TD and TSR and 92 tables for FA.

Recently, larger datasets [3,9, 22, 23] have been created
by collecting crowd-sourced table annotations automatically
from existing documents. We summarize these datasets in
Tab. 1. Each source table has an annotation for its content
and structure in a markup format such as HTML, XML, or
LaTeX. Various methods are used to determine each table's
spatial location within its containing document to create
a correspondence between its markup and its presentation.
From there, datasets commonly frame the TSR task as: given
an input table, output its cell structure—the assignment of
cells to rows and columns and the text content for each
cell, with image and HTML being example input and output
formats, respectively, for these.

More recently, two large datasets, FinTabNet and an en-
hanced version of PubTabNet, have added location infor-
mation for cells, similar to ICDAR-2013. Adding location
information enables the TSR task to be framed as outputting
cell location instead of cell content, with cell content extrac-
tion being a trivial subsequent step. This increases the range
of possible supervised modeling approaches. However, the
bounding boxes for cells defined by these datasets cover only
the text portion of each and exclude any additional whites-
pace a cell might contain. This has a few implications, such
as making bounding boxes for blank cells undefined and
excluding attributes contributed by whitespace, such as text
indentation and alignment. Therefore, one question left open
by prior work is how to define bounding boxes for all cells,
including blank cells.

There are additional challenges related to annotation com-
pleteness and quality that have not been addressed by prior
datasets. In terms of completeness, prior large-scale datasets
have also not included bounding boxes for rows and columns.
Additionally, most datasets do not annotate the column
header, and no prior large-scale dataset exists that speci-
fies the row header of a table. This not only limits the range
of modeling approaches that can be applied to TSR but limits
how completely the overall TE task can be solved.

Another open challenge is automated verfication and mea-
surement of annotation quality, which is important due to the
impracticality of verifying large-scale annotations manually.
Prior datasets have also not addressed the significant issue of
oversegmented annotations. These are important issues, as
noise and mistakes in training data potentially harm learning
and in evaluation data potentially lead to an underestimate of
model performance. But currently the extent to which these
issues affect model training and evaluation is unexplored.

Modeling approaches One of the most common mod-
eling approaches for TSR is to frame the task as some
form of object detection [14, 17, 22]. Other approaches
include those based on image-to-text [9] and graph-based
approaches [3,15]. While a number of general-purpose archi-
tectures, such as Faster R-CNN [16], exist for these model
patterns, the unique characteristics of table and the relative
lack of training data have both contributed to the commonly
observed underperformance of these models when applied
to TSR out-of-the-box.

To get around deficiencies in training data, some ap-
proaches model TSR in ways that are only partial solutions
to the task, such as row and column detection in Deep-
DeSRT [17], which ignores spanning cells, or image-to-
markup without cell text content, as in models trained on
TableBank [9]. Other approaches use custom pipelines that
branch to consider different cases separately, such as train-
ing separate models to recognize tables with and without
visible borders surrounding every cell [14, 22]. Many of the
previously mentioned approaches also use engineered model

Table 1. Comparison of crowd-sourced datasets for table structure recognition.

| Dataset | Input Modality | # Tables | Cell Topology | Cell Content | Cell Location | Row & Column Location | Canonical Structure |
| ----------- | ----------- | ----------- | ----------- | ----------- | ----------- | ----------- | ----------- |
| TableBank [9] | Image | 145K | ✓ | ✓ | | | |
| SciTSR [3] | PDF* | 15K | ✓ | ✓ | | | |
| PubTabNet [22,23] | Image | 510K+ | ✓ | ✓ | ✓ | | |
| FinTabNet [22] | PDF* | 113K | ✓ | ✓ | ✓ | | |
| PubTables-1M (ours) | PDF* | 948K | ✓ | ✓ | ✓ | ✓ | ✓ |

*Multiple input modalities, such as image or text, can be derived from annotated PDF data.
+The authors release annotations for 510K of the 568K total tables in their dataset.
For these datasets, cell bounding boxes are given for non-blank cells only and exclude any non-text portion of a cell.

components or custom training procedures, and incorporate
rules or other unlearned processing stages tailored to the TSR
task, which brings in prior knowledge to lessen the burden
placed on learning the task from data. Currently, no solution
exists that uses a simple supervised learning approach with
an off-the-shelf architecture, solves the TSR task completely,
and achieves state-of-the-art performance.

3. PubTables-1M

In this section, we describe the process used to develop
PubTables-1M. First, to obtain a large source of annotated
tables, we choose the PMCOA corpus, which consists of
millions of publicly available scientific articles. In the PM-
COA corpus, each scientific article is given in two forms: as
a PDF document, which visually presents the article, and as
an XML document, which provides a semantic description
and hierarchical organization of the document's elements.
Each table's content and structure is specified using standard
HTML tags.

However, because this data was not intended for use as
ground truth for table extraction modeling, it does not explic-
itly label or guarantee multiple things that would be helpful
for this purpose. For instance, although the same tables ap-
pear in both documents, no direct correspondence between
them is given, nor the spatial location of each table. In terms
of data quality, while tables are generally annotated reliably,
it is not guaranteed that column headers are annotated com-
pletely or that text content as annotated exactly matches the
text content as it appears in the PDF. Finally, some labels,
such as the row header for each table, are not annotated at
all.

The basic approach we take to overcome these issues is
first we attempt to reliably infer as much missing annotation
information as possible (for instance, the spatial location
of each table) from the information that is present, then we
verify that each annotation meets certain requirements for
consistency. In some cases, we correct an annotation to
attempt to make it more consistent, such as merging cells
that are oversegmented. We consider certain requirements
for tables to be strict and samples whose annotations violate
these are removed. This provides a set of conditions for
quality and consistency that the annotations are guaranteed to
meet. In the rest of this section, we describe these conditions
and the steps we take to derive ground truth that meets them.

Alignment Text in a PDF document has spatial location
[Xmin, Ymin, Xmax, Ymax], while text in an XML document ap-
pears inside semantically labeled tags. Because the cor-
respondence between these is not given, the first step in
creating PubTables-1M is to match the text content from
both. We process the PDF document into a sequence of
characters each with their associated bounding box and use
the Needleman-Wunsch algorithm [10] to align this with the
character sequence for the text extracted from each table
HTML. This connects the text within each HTML tag to its
spatial location with the PDF document. For each cell with
text, we compute the union of the bounding boxes for each
character of the cell's text, which we refer to as a text cell
bounding box.

Completion Following alignment, we complete the spatial
annotations to define bounding boxes for rows, columns, and
the entire table. The bounding box for the table is defined
simply as the union of all text cell bounding boxes. The Xmin
and Xmax of the bounding box for each row are defined as
the xmin and Xmax of the table, giving every row the same
horizontal length. The Ymin and ymax of the bounding box for
each row, m, are defined as the Ymin and Ymax of the union
of the text cells for each cell whose starting row or ending
row is m. Similarly, the Ymin and ymax of the bounding box
for each column are defined as the Ymin and Ymax of the table.
The Xmin and Xmax of the bounding box for each column,
n, are defined as the Xmin and Xmax of the union of the text
cell for each cell whose starting column or ending column
is n. From these definitions, the grid cell for each cell is
defined as the union of the bounding boxes of the cell's
rows intersected with the union of the bounding boxes for
its columns. Unlike the text cell, the grid cell is defined even
for blank cells.

Canonicalization The primary goal of the canonicaliza-
tion step is to correct oversegmentation in a table's structure
annotations. To do this, we need to make assumptions about
a table's intended structure. As the canonicalization algo-
rithm itself is relatively simple, we first describe it, then
detail the assumptions that motivate it. Put simply, canoni-
calization amounts to merging adjacent cells under certain
conditions. This algorithm is given in Algorithm 1. But
because it only operates on cells in the headers, HTML does
not have a tag for specifying a table's row header, and we
observed that the column headers for tables in the PMCOA
corpus are not always correct, we also include steps for in-
ferring additional header cells that we believe can be reliably
inferred in PMCOA markup annotations. These additional
steps significantly increase the number of cells whose over-
segmentation we are able to correct.

Algorithm 1 PubTables-1M Canonicalization
1: ADD CELLS TO THE COLUMN AND ROW HEADERS
2: Split every blank spanning cell into blank grid cells
3: if the first row starts with a blank cell then add the first row
to the column header
4: if there is at least one row labeled as part of the column
header then
5: while every column in the column header does not have
at least one complete cell that only spans that column do:
6: add the next row to the column header
7: end if
8: for each row do: if the row is not in the column header and
has exactly one non-blank cell that occupies the first column
then label it a projected row header
9: if any cell in the first column below the column header is
a spanning cell or blank then add the column (below the
column header) to the row header
10: MERGE CELLS
11: for each cell in the column header do recursively merge the
cell with any adjacent cells above and below in the column
header that span the exact same columns
12: for each cell in the column header do recursively merge the
cell with any adjacent blank cells below it if every adjacent
cell below it is blank and in the column header
13: for each cell in the column header do recursively merge the
cell with any adjacent blank cells above it if every adjacent
cell above it is blank
14: for each projected row header do merge all of the cells in
the row into a single cell
15: for each cell in the row header do recursively merge the cell
with any adjacent blank cells below it

We first assume that each table has an intended structure
consistent with the Wang model [21], which in a study Wang
found was true for 97 percent of observed tables. Under
this model, the headers of the table each have a hierarchical
structure that corresponds logically to a tree. We assert that
for a structure annotation to be consistent with a table's
logical structure, there should be exactly one cell for every
tree node. We also assume that each value in the table is
indexed by a unique set of keys. We interpret this to mean
that each column in the body of the table corresponds to a
unique leaf node in the column header tree, and similarly that
each row in the body corresponds to a unique leaf node in the
row header tree (the index of a row or column can serve as a
key if necessary). These assumptions enable us to determine
if a row or column header is only partially annotated and if
so, to extend it to additional columns or rows, respectively.
However, to keep the precision of the algorithm high, for
row headers we only attempt to infer projected row headers
(PRHs, also known as projected multi-level row headers [8],
section headers [13], or super-rows [20]) and to infer cells
that are in the first column of the header. The PRHs of a
table can be identified using the rule in Line 7. Inference of
the full row header is considered outside the scope of this
work.

We also assume that any internal node in a header tree has
at least two children. If not, ambiguity could arise in a table's
logical structure because an internal node could optionally
be split into a parent node and a single child node. The final
assumptions we make are in regards to the root cause of
oversegmentation in markup annotations. We assume that
cells will only be oversegmented if an oversegmentation
is consistent with the table's appearance. In practice, this
means that cells with centered text will not be oversegmented
in the direction of the alignment because this is likely to alter
the table's appearance. For non-centered text, we expect
that when cells in either header are oversegmented, this
will happen vertically, as in Fig. 3b, and not horizontally
due to the fact that text fills horizontal space before it fills
vertical space, leaving more vertical space unused. Further,
we expect that oversegmented cells in the row header will
have text that is top aligned. Finally, we expect that when
projected row headers are oversegmented, this will happen
horizontally, not vertically, as a projected row header already
occupies only one row.

Finally, there are two additional cases that we must handle
by convention. One case is when one or more rows of blank
grid cells are between a parent cell and all of its children
cells in the column header. In this case, we can choose either
to merge all of the blank cells with the parent cell above
it or each with the child cell below it, and we choose the
convention to merge all of the blank cells with the child,
which occurs in Line 10. The final case is when a table has
an blank stub head (according to the Wang model) in its
top-left corner. In this case, the blank cells are not part of the
table, so the assumptions about table structure do not suggest
how they should be grouped. We choose by convention to
merge all blank cells in the same column in a blank stub
head, which is consistent with the scheme in Line 10.

Table 2. Estimated measure of oversegmentation for projected row headers (PRHs) by dataset. As PRHs are only one type of cell that can be
oversegmented, this is a partial survey of the total oversegmentation in these datasets.

| Dataset | Total Tables Investigated | Total Tables with a PRH* | Tables with an oversegmented PRH | |
| ----------- | ----------- | ----------- | ----------- | ----------- |
| | | | Total | % (of total with a PRH) | % (of total investigated) |
| SciTSR | 10,431 | 342 | 54 | 15.79% | 0.52% |
| PubTabNet | 422,491 | 100,159 | 58,747 | 58.65% | 13.90% |
| FinTabNet | 70,028 | 25,637 | 25,348 | 98.87% | 36.20% |
| PubTables-1M (ours) | 761,262 | 153,705 | 0 | 0% | 0% |

+ We exclude tables with fewer than five rows; to avoid column header rows we skip the first four rows when searching for PRHs.
*PRH = projected row header; these can be reliably detected in datasets without any prior row or column header annotations.

Limitations While the stated goal of canonicalization is
applicable to any table structure annotation, we note that
Algorithm 1 is designed to achieve this specifically for the
annotations in the PMCOA dataset. Canonicalizing tables
from other datasets may require additional assumptions and
is considered outside the scope of this work. Finally, it
should be noted that canonicalization does not guarantee
mistake-free annotations. Remaining issues are addressed
using the automated quality control procedure described
next.

Quality control Because PubTables-1M is too large to be
verified manually, we check for potential errors automatically
and filter these from the data. First, as tables rendered from
markup should not contain overlapping rows or overlapping
columns, we discard any table where this occurs, as these are
likely due to mistakes introduced by the alignment process.
To filter out mistakes made both by the original annotators
and our automated processing, we compare the edit distance
between the non-whitespace text for every cell in the original
XML annotations with the text extracted from the PDF inside
the grid cell bounding box. We filter out any tables for which
the normalized edit distance between these averaged over
every cell is above 0.05. We do not force the text from
each to be exactly equal, as the PDF text can differ even
when everything is annotated correctly, due to things like
word wrapping, which may add hyphens that are not in the
source annotations. When the annotations do slightly differ
from their corresponding PDF text, we choose to consider
the PDF text to be the ground truth. As tables with correct
location information provide an unambiguous assignment of
all words in the table to cells, we also compute the average
fraction of overlap between each word appearing within the
boundary of the table and its most overlapping grid cell, and
discard tables with an average below 0.9. Finally, we remove
outliers by counting the number of objects in a table (defined
in Sec. 4) and removing tables with more than 100. In all,
less than 0.1% of tables are discarded as outliers.

PubTables-1M is the first dataset that verifies annotations
at the cell level and provides a measurable assurance of
consistency for the ground truth. This shows that improving
the explicitness of information is valuable in part because it
leads to more opportunities for catching inconsistencies and
errors embedded within the data.

Dataset statistics and splits In total, PubTables-1M con-
tains 947,642 tables for TSR, of which 52.7% are complex
(have at least one spanning cell). Prior to canonicalization,
only 40.1% of the tables in the set were considered com-
plex by the original annotators. Canonicalization adjusts the
annotations in some way for 34.7% of tables, or 65.8% of
complex tables.

To further assess the impact on oversegmentation, we
compare our final dataset with other datasets in Tab. 2. Pre-
cisely measuring the amount of oversegmentation in a dataset
requires annotations for the row and column headers. But
because other datasets lack these, we instead measure just
oversegmented projected row headers (PRHs). PRHs can
be detected reliably without explicit annotations using the
rule in Line 7. To account for missing column header an-
notations, we do not start looking for PRHs until at least
the fifth row, which assumes the column header occupies at
most four rows, and we simply exclude any tables that have
fewer than five rows. In case there are un-annotated footers,
we also do not count any detected PRHs that are the last
rows of the table. A detected PRH is oversegmented if its
row contains a blank cell. As can be seen, canonicalization
eliminates a significant source of oversegmentation, and thus
ambiguity, that is present in other datasets. Interesting to
note, FinTabNet nearly always oversegments projected row
headers. While self-consistent, this widespread oversegmen-
tation contradicts the logical structure of the table and causes
potential issues with combining this dataset with others that
annotate these rows differently.

We split PubTables-1M randomly into train, validation,
and test sets at the document level using an 80/10/10 split.
For TSR, this results in 758,849 tables for training; 94,959
for validation; and 93,834 for testing. For TD, there are
460,589 fully-annotated pages containing tables for training;
57,591 for validation; and 57,125 for testing. An example

<!-- image -->
Table 3. Test performance of models on PubTables-1M using object
detection metrics.

| Task | Model | AP | AP50 | AP75 | AR |
| ----------- | ----------- | ----------- | ----------- | ----------- | ----------- |
| TD | Faster R-CNN | 0.825 | 0.985 | 0.927 | 0.866 |
| | DETR | 0.966 | 0.995 | 0.988 | 0.981 |
| TSR + FA | Faster R-CNN | 0.722 | 0.815 | 0.785 | 0.762 |
| | DETR | 0.812 | 0.912 | 0.971 | 0.948 |

<!-- image -->
Figure 4. An example table with dilated bounding box annotations
for different object classes for jointly modeling table structure
recognition and functional analysis.

page and table annotation for TD is shown in Fig. 2. Note
that tables that span multiple pages are considered outside
the scope of this work.

4. Proposed Model

We model all three tasks of TD, TSR, and FA as object
detection with images as input. For TD, we use two object
classes: table and table rotated. The table rotated class
corresponds to tables that are rotated counterclockwise 90
degrees.

TSR and FA model We use a novel approach that models
TSR and FA jointly using six object classes: table, table
column, table row, table column header, table projected row
header, and table spanning cell. We illustrate these classes
in Fig. 4. The intersection of each pair of table column
and table row objects can be considered to form a seventh
implicit class, table grid cell. These objects model a table's
hierarchical structure through physical overlap.

For the TSR and FA model, we use bounding boxes that
are dilated. To create dilated bounding boxes, for each pair
of adjacent rows and each pair of adjacent columns, we
expand their boundaries until they meet halfway, which fills
the empty space in between them. Similarly we expand the
objects from the other classes so their boundaries match
the adjustments made to the rows and columns they occupy.
After, there are no gaps or overlap between rows, between
columns, or between cells.

To demonstrate the proposed dataset and the object detec-
tion modeling approach, we apply the Detection Transformer
(DETR) [2] to all three TE tasks. We train one DETR model
for TD and one DETR model for both TSR and FA. For
comparison, we also train a Faster R-CNN [16] model for
the same tasks. All models use a ResNet-18 backbone pre-
trained on ImageNet with the first few layers frozen. We
avoid custom engineering the models and training proce-
dures for each task, using default settings wherever possible
to allow the data to drive the result.

5. Experiments

In this section, we report the results of training the pro-
posed models on data derived from PubTables-1M. For TD,
we train two models: DETR and Faster R-CNN. We report
the results in Tab. 3. For table detection, DETR slightly
outperforms Faster R-CNN on AP50 but significantly out-
performs on AP. We interpret this to mean that while both
models are able to learn to detect tables, DETR precisely
localizes tables much better than Faster R-CNN.

For TSR and FA, we train three models: Faster R-CNN
and DETR on the canonicalized data, and DETR on the origi-
nal, non-canonical (NC) annotations (DETR-NC). We report
the results using object detection metrics for the models
trained on canonical data in Tab. 3, which measures per-
formance jointly on TSR and FA, and report results for all
models using TSR-only metrics in Tab. 4. For TSR only,
we also evaluate DETR-NC on both the canonical and the
original non-canonical test data.

For assessing TSR performance, we report the table con-
tent accuracy metric (Acccont), which is the percentage of
tables whose text content matches the ground truth exactly
for every cell, as well as several metrics for partial table
correctness, which use different strategies to give credit for
correct cells when not all cells are correct. For partial correct-
ness, we use the F-score of the standard adjacent cell content
metric [5] and the recently proposed GriTS metrics [19].
GriTS metrics have the form,

```
GriTSf (A, B) = 2 * Σij f(Ai,j, Βi,j) / |A| + |B| ,
```

which can also be interpreted as an F-score. GriTS represents
the ground truth and predicted tables as matrices, A and B,
and computes a similarity score between the most similar
substructures [1] of these matrices, A and B, where a sub-
structure is defined as a selection of m rows and n columns
from the matrix. Compared to other metrics for TSR, this for-
mulation better captures the two-dimensional structure and

Table 4. Test performance of the TSR + FA models on PubTables-1M on TSR metrics.

| Test Data | Model | Table Category | Acccont | GriTSTop | GriTSCont | GriTSLoc | AdjCont |
| ----------- | ----------- | ----------- | ----------- | ----------- | ----------- | ----------- | ----------- |
| Non-Canonical | DETR-NC | Simple | 0.8678 | 0.9872 | 0.9859 | 0.9821 | 0.9801 |
| | | Complex | 0.5360 | 0.9600 | 0.9618 | 0.9444 | 0.9505 |
| | | All | 0.7336 | 0.9762 | 0.9761 | 0.9668 | 0.9681 |
| Canonical | DETR-NC | Simple | 0.9349 | 0.9933 | 0.9920 | 0.9900 | 0.9865 |
| | | Complex | 0.2712 | 0.9257 | 0.9290 | 0.9044 | 0.9162 |
| | | All | 0.5851 | 0.9576 | 0.9588 | 0.9449 | 0.9494 |
| | Faster R-CNN | Simple | 0.0867 | 0.8682 | 0.8571 | 0.6869 | 0.8024 |
| | | Complex | 0.1193 | 0.8556 | 0.8507 | 0.7518 | 0.7734 |
| | | All |
