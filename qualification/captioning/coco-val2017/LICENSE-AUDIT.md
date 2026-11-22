# License, evaluation-use and overlap audit — COCO val2017 captioning corpus

Manifest: `manifest.json` (30 images, digest
`1bf64e698dbc64ea7a2c3ecbe1ceb6596bd0e1edfa3525810e45065c2f721ba6`).

## 1. License and evaluation use

- **Annotations:** COCO 2017 captions are published by the COCO Consortium and distributed at
  <https://cocodataset.org/#termsofuse> under CC BY 4.0. The annotation archive itself carries no
  license field; the audit records the upstream claim, not a relicense. Attribution is the
  citation in the manifest (`Lin et al., Microsoft COCO, ECCV 2014`).
- **Images:** COCO images are Flickr-hosted photographs. The COCO terms permit use for
  evaluation and research. They are **not redistributed** here: the repository stores only
  digests, file names and reference captions, and the acquisition script downloads each image
  from the pinned public URL for the duration of the run.
- **What is committed:** 30 reference caption sets (five short sentences each) and image digests.
  That is a de minimis excerpt used to score captions, not a redistribution of the photographs.
- **Residual risk:** the copyright of each photograph remains with its Flickr author. This corpus
  is evaluation-only and must not be republished as an image dataset.

## 2. Training overlap — the important caveat

COCO val2017 is a standard evaluation split, but it is **not** a blind hold-out for these models:

- SmolVLM is trained on The Cauldron and Docmatix, and its SigLIP/SmolLM2 backbones are pretrained
  on large web corpora that may include COCO images or captions.
- Florence-2 is trained on FLD-5B, which includes web-scale image-text pairs and standard
  detection/captioning corpora.

Neither training set can be inspected from here, so **overlap cannot be excluded**. Consequences
that are stated in the report rather than papered over:

- the automatic caption metrics measure whether the SDK pipeline produces usable captions for
  these models on this hardware, and nothing about generalization;
- a high recall of reference terms does not show that the model saw the image rather than the
  training distribution;
- **no captioning go/no-go can rest on these numbers** — the preregistration already makes the
  verdict `pending_human` because visible-fact precision needs blind raters.

The number-bearing stratum was chosen for measurement power, not because those images are harder.

## 3. What the references support

Five short references per image were written independently by crowd workers. They support:

- **presence** checks: is a number/term from the references in the generated caption;
- **omission** proxies: how much reference content is missing.

They do **not** support precision: a correct caption fact absent from all five references is not
an invention, and a hallucinated relation can reuse reference words. The scorer therefore reports
invented *numbers* as unverified candidates and refuses to compute a fact-precision number.
