import * as React from "react"
import {
  AlignBoxBottomCenterIcon,
  Heading01Icon,
  ImageCompositionIcon,
  LeftToRightListBulletIcon,
  ParagraphIcon,
  Table01Icon,
  TextCenterlineCenterTopIcon,
  TextNumberSignIcon,
} from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import { useVirtualizer } from "@tanstack/react-virtual"
import ReactMarkdown from "react-markdown"
import rehypeRaw from "rehype-raw"
import rehypeSanitize, { defaultSchema } from "rehype-sanitize"
import remarkGfm from "remark-gfm"

import { cn } from "@/lib/utils"
import { ScrollArea } from "@/components/ui/scroll-area"

export type Point = {
  x: number
  y: number
}

export type BoundingBox = {
  left: number
  top: number
  right: number
  bottom: number
}

export type OcrBlockType =
  | "heading"
  | "paragraph"
  | "list"
  | "table"
  | "figure"
  | "header"
  | "footer"
  | "page_number"

export type ParsedOcrBlock = {
  id: string
  type: string
  content: string
  metadata: {
    page: {
      number: number
      width: number
      height: number
    }
    layoutClass?: string
    minOcrConfidence?: number
    avgOcrConfidence?: number
  }
  polygon?: Point[]
  boundingBox?: BoundingBox
}

export type ParsedOcrOutput = {
  chunks: {
    blocks: ParsedOcrBlock[]
  }[]
}

export type OcrBlock = {
  id: string
  type: OcrBlockType
  text: string
  page: number
  pageWidth: number
  pageHeight: number
  confidence: number
  polygon?: Point[]
  boundingBox?: BoundingBox
}

export type HighlightArea = {
  left: number
  top: number
  width: number
  height: number
}

export const PDF_URL = "/samples/attention.pdf"
const OCR_BLOCK_ROW_MIN_ESTIMATE = 92
const OCR_BLOCK_ROW_VERTICAL_CHROME = 62
const OCR_BLOCK_LINE_HEIGHT = 20
const OCR_BLOCK_ESTIMATED_CHARS_PER_LINE = 42
const OCR_BLOCK_ROW_GAP = 8
const OCR_BLOCK_LIST_PADDING = 12
const OCR_MARKDOWN_SCHEMA = {
  ...defaultSchema,
  attributes: {
    ...defaultSchema.attributes,
    figure: [...(defaultSchema.attributes?.figure ?? []), "type"],
  },
  tagNames: [
    ...(defaultSchema.tagNames ?? []),
    "caption",
    "figcaption",
    "figure",
  ],
}
const OCR_MARKDOWN_REHYPE_PLUGINS: NonNullable<
  React.ComponentProps<typeof ReactMarkdown>["rehypePlugins"]
> = [rehypeRaw, [rehypeSanitize, OCR_MARKDOWN_SCHEMA]]
const OCR_MARKDOWN_REMARK_PLUGINS: NonNullable<
  React.ComponentProps<typeof ReactMarkdown>["remarkPlugins"]
> = [remarkGfm]
const OCR_MARKDOWN_FIGURE_CAPTION_PATTERN = /<\/?caption>/gi

function getEstimatedOcrBlockRowHeight(block: OcrBlock) {
  const plainText = block.text
    .replace(OCR_MARKDOWN_FIGURE_CAPTION_PATTERN, "")
    .replace(/<[^>]*>/g, "")
    .replace(/\s+/g, " ")
    .trim()
  const explicitLineCount = block.text.split(/\n+/).filter(Boolean).length
  const wrappedLineCount = Math.max(
    explicitLineCount,
    Math.ceil(plainText.length / OCR_BLOCK_ESTIMATED_CHARS_PER_LINE)
  )
  const contentHeight = Math.max(1, wrappedLineCount) * OCR_BLOCK_LINE_HEIGHT
  const typeExtraHeight =
    block.type === "figure" ? 44 : block.type === "table" ? 28 : 0

  return Math.max(
    OCR_BLOCK_ROW_MIN_ESTIMATE,
    OCR_BLOCK_ROW_VERTICAL_CHROME +
      contentHeight +
      typeExtraHeight +
      OCR_BLOCK_ROW_GAP
  )
}

export const ATTENTION_OCR_OUTPUT = {
  chunks: [
    {
      blocks: [
        {
          id: "block_1_IKDP4i",
          type: "header",
          content: "arXiv:1706.03762v7 [cs.CL] 2 Aug 2023",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Header",
            minOcrConfidence: 0.959,
            avgOcrConfidence: 0.98,
          },
          polygon: [
            {
              x: 36.16179897837395,
              y: 447.38040863840206,
            },
            {
              x: 75.85163708150822,
              y: 447.38040863840206,
            },
            {
              x: 75.85163708150822,
              y: 1155.1796566956025,
            },
            {
              x: 36.16179897837395,
              y: 1155.1796566956025,
            },
          ],
          boundingBox: {
            left: 36.16179897837395,
            top: 447.38040863840206,
            right: 75.85163708150822,
            bottom: 1155.1796566956025,
          },
        },
        {
          id: "block_1_rtNdsr",
          type: "header",
          content:
            "Provided proper attribution is provided, Google hereby grants permission to reproduce the tables and figures in this paper solely for use in journalistic or scholarly works.",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Header",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 258.50183528705236,
              y: 151.7837261257315,
            },
            {
              x: 1016.8830982960054,
              y: 151.7837261257315,
            },
            {
              x: 1016.8830982960054,
              y: 233.84929126366637,
            },
            {
              x: 258.50183528705236,
              y: 233.84929126366637,
            },
          ],
          boundingBox: {
            left: 258.50183528705236,
            top: 151.7837261257315,
            right: 1016.8830982960054,
            bottom: 233.84929126366637,
          },
        },
        {
          id: "block_1_eo9AoV",
          type: "heading",
          content: "# Attention Is All You Need",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Heading",
            minOcrConfidence: 0.99,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 440.0215455215343,
              y: 312.01490975860366,
            },
            {
              x: 831.4373211268961,
              y: 312.01490975860366,
            },
            {
              x: 831.4373211268961,
              y: 339.0022611402928,
            },
            {
              x: 440.0215455215343,
              y: 339.0022611402928,
            },
          ],
          boundingBox: {
            left: 440.0215455215343,
            top: 312.01490975860366,
            right: 831.4373211268961,
            bottom: 339.0022611402928,
          },
        },
        {
          id: "block_1_Kdt5Cu",
          type: "text",
          content: "Ashish Vaswani* Google Brain avaswani@google.com",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.977,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 242.9606625633518,
              y: 490.4495616712068,
            },
            {
              x: 449.64758323056856,
              y: 490.4495616712068,
            },
            {
              x: 449.64758323056856,
              y: 555.9028907431696,
            },
            {
              x: 242.9606625633518,
              y: 555.9028907431696,
            },
          ],
          boundingBox: {
            left: 242.9606625633518,
            top: 490.4495616712068,
            right: 449.64758323056856,
            bottom: 555.9028907431696,
          },
        },
        {
          id: "block_1_6Xv6uR",
          type: "text",
          content: "Llion Jones* Google Research llion@google.com",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.927,
            avgOcrConfidence: 0.982,
          },
          polygon: [
            {
              x: 264.3723327748097,
              y: 594.5307239733245,
            },
            {
              x: 438.3605845653227,
              y: 594.5307239733245,
            },
            {
              x: 438.3605845653227,
              y: 660.7685646688132,
            },
            {
              x: 264.3723327748097,
              y: 660.7685646688132,
            },
          ],
          boundingBox: {
            left: 264.3723327748097,
            top: 594.5307239733245,
            right: 438.3605845653227,
            bottom: 660.7685646688132,
          },
        },
        {
          id: "block_1_Lx35WU",
          type: "text",
          content: "Noam Shazeer* Google Brain noam@google.com",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.983,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 479.08373644752226,
              y: 490.1169480632122,
            },
            {
              x: 643.7573955006842,
              y: 490.1169480632122,
            },
            {
              x: 643.7573955006842,
              y: 556.2343205043247,
            },
            {
              x: 479.08373644752226,
              y: 556.2343205043247,
            },
          ],
          boundingBox: {
            left: 479.08373644752226,
            top: 490.1169480632122,
            right: 643.7573955006842,
            bottom: 556.2343205043247,
          },
        },
        {
          id: "block_1_VxOJIL",
          type: "text",
          content:
            "Aidan N. Gomez* * University of Toronto aidan@cs.toronto.edu",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.095,
            avgOcrConfidence: 0.875,
          },
          polygon: [
            {
              x: 489.9820146769503,
              y: 593.5908916444707,
            },
            {
              x: 708.1676900821881,
              y: 593.5908916444707,
            },
            {
              x: 708.1676900821881,
              y: 657.7173651788468,
            },
            {
              x: 489.9820146769503,
              y: 657.7173651788468,
            },
          ],
          boundingBox: {
            left: 489.9820146769503,
            top: 593.5908916444707,
            right: 708.1676900821881,
            bottom: 657.7173651788468,
          },
        },
        {
          id: "block_1_m1Gb17",
          type: "text",
          content: "Niki Parmar* Google Research nikip@google.com",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.99,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 673.3537325893875,
              y: 490.2837284059453,
            },
            {
              x: 848.6601168221802,
              y: 490.2837284059453,
            },
            {
              x: 848.6601168221802,
              y: 556.287735673718,
            },
            {
              x: 673.3537325893875,
              y: 556.287735673718,
            },
          ],
          boundingBox: {
            left: 673.3537325893875,
            top: 490.2837284059453,
            right: 848.6601168221802,
            bottom: 556.287735673718,
          },
        },
        {
          id: "block_1_otnY8T",
          type: "text",
          content: "\u0141ukasz Kaiser* Google Brain lukaszkaiser@google.com",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.984,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 759.3324257509552,
              y: 594.2661578816579,
            },
            {
              x: 1011.1333304078039,
              y: 594.2661578816579,
            },
            {
              x: 1011.1333304078039,
              y: 660.9298993160851,
            },
            {
              x: 759.3324257509552,
              y: 660.9298993160851,
            },
          ],
          boundingBox: {
            left: 759.3324257509552,
            top: 594.2661578816579,
            right: 1011.1333304078039,
            bottom: 660.9298993160851,
          },
        },
        {
          id: "block_1_ICaos6",
          type: "text",
          content: "Illia Polosukhin* * illia.polosukhin@gmail.com",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.107,
            avgOcrConfidence: 0.766,
          },
          polygon: [
            {
              x: 495.9563429338219,
              y: 697.325802423004,
            },
            {
              x: 778.5449758933407,
              y: 697.325802423004,
            },
            {
              x: 778.5449758933407,
              y: 742.1158899292909,
            },
            {
              x: 495.9563429338219,
              y: 742.1158899292909,
            },
          ],
          boundingBox: {
            left: 495.9563429338219,
            top: 697.325802423004,
            right: 778.5449758933407,
            bottom: 742.1158899292909,
          },
        },
        {
          id: "block_1_zNN0g5",
          type: "section_heading",
          content: "### Abstract",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.995,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 590.8429918498018,
              y: 804.3203170891095,
            },
            {
              x: 683.8031267597728,
              y: 804.3203170891095,
            },
            {
              x: 683.8031267597728,
              y: 823.7656170264223,
            },
            {
              x: 590.8429918498018,
              y: 823.7656170264223,
            },
          ],
          boundingBox: {
            left: 590.8429918498018,
            top: 804.3203170891095,
            right: 683.8031267597728,
            bottom: 823.7656170264223,
          },
        },
        {
          id: "block_1_Pzc7ia",
          type: "text",
          content:
            "The dominant sequence transduction models are based on complex recurrent or convolutional neural networks that include an encoder and a decoder. The best performing models also connect the encoder and decoder through an attention mechanism. We propose a new simple network architecture, the Transformer, based solely on attention mechanisms, dispensing with recurrence and convolutions entirely. Experiments on two machine translation tasks show these models to be superior in quality while being more parallelizable and requiring significantly less time to train. Our model achieves 28.4 BLEU on the WMT 2014 English- to-German translation task, improving over the existing best results, including ensembles, by over 2 BLEU. On the WMT 2014 English-to-French translation task, our model establishes a new single-model state-of-the-art BLEU score of 41.8 after training for 3.5 days on eight GPUs, a small fraction of the training costs of the best models from the literature. We show that the Transformer generalizes well to other tasks by applying it successfully to English constituency parsing both with large and limited training data.",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.987,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 298.2268201173657,
              y: 861.5167879950731,
            },
            {
              x: 978.0244980415288,
              y: 861.5167879950731,
            },
            {
              x: 978.0244980415288,
              y: 1201.6481075430274,
            },
            {
              x: 298.2268201173657,
              y: 1201.6481075430274,
            },
          ],
          boundingBox: {
            left: 298.2268201173657,
            top: 861.5167879950731,
            right: 978.0244980415288,
            bottom: 1201.6481075430274,
          },
        },
        {
          id: "block_1_bHnLKj",
          type: "text",
          content:
            "*Equal contribution. Listing order is random. Jakob proposed replacing RNNs with self-attention and started the effort to evaluate this idea. Ashish, with Illia, designed and implemented the first Transformer models and has been crucially involved in every aspect of this work. Noam proposed scaled dot-product attention, multi-head attention and the parameter-free position representation and became the other person involved in nearly every detail. Niki designed, implemented, tuned and evaluated countless model variants in our original codebase and tensor2tensor. Llion also experimented with novel model variants, was responsible for our initial codebase, and efficient inference and visualizations. Lukasz and Aidan spent countless long days designing various parts of and implementing tensor2tensor, replacing our earlier codebase, greatly improving results and massively accelerating our research.",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.951,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 222.8755505415645,
              y: 1247.2573697082978,
            },
            {
              x: 1049.6971241749116,
              y: 1247.2573697082978,
            },
            {
              x: 1049.6971241749116,
              y: 1429.7100451476592,
            },
            {
              x: 222.8755505415645,
              y: 1429.7100451476592,
            },
          ],
          boundingBox: {
            left: 222.8755505415645,
            top: 1247.2573697082978,
            right: 1049.6971241749116,
            bottom: 1429.7100451476592,
          },
        },
        {
          id: "block_1_J3Olku",
          type: "text",
          content: "+ Work performed while at Google Brain.",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.11,
            avgOcrConfidence: 0.869,
          },
          polygon: [
            {
              x: 250.1677060649343,
              y: 1434.3817888991277,
            },
            {
              x: 555.1793119333087,
              y: 1434.3817888991277,
            },
            {
              x: 555.1793119333087,
              y: 1456.532320926064,
            },
            {
              x: 250.1677060649343,
              y: 1456.532320926064,
            },
          ],
          boundingBox: {
            left: 250.1677060649343,
            top: 1434.3817888991277,
            right: 555.1793119333087,
            bottom: 1456.532320926064,
          },
        },
        {
          id: "block_1_aQsakB",
          type: "text",
          content: "#Work performed while at Google Research.",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.652,
            avgOcrConfidence: 0.94,
          },
          polygon: [
            {
              x: 249.36288151427775,
              y: 1458.4100914288283,
            },
            {
              x: 582.4047478446125,
              y: 1458.4100914288283,
            },
            {
              x: 582.4047478446125,
              y: 1479.773507369192,
            },
            {
              x: 249.36288151427775,
              y: 1479.773507369192,
            },
          ],
          boundingBox: {
            left: 249.36288151427775,
            top: 1458.4100914288283,
            right: 582.4047478446125,
            bottom: 1479.773507369192,
          },
        },
        {
          id: "block_1_iKp4Ae",
          type: "footer",
          content:
            "31st Conference on Neural Information Processing Systems (NIPS 2017), Long Beach, CA, USA.",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Footer",
            minOcrConfidence: 0.99,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 224.27405893367575,
              y: 1527.1945287374626,
            },
            {
              x: 958.2568426201814,
              y: 1527.1945287374626,
            },
            {
              x: 958.2568426201814,
              y: 1546.3855383736748,
            },
            {
              x: 224.27405893367575,
              y: 1546.3855383736748,
            },
          ],
          boundingBox: {
            left: 224.27405893367575,
            top: 1527.1945287374626,
            right: 958.2568426201814,
            bottom: 1546.3855383736748,
          },
        },
        {
          id: "block_1_zPHQaR",
          type: "text",
          content: "Jakob Uszkoreit* Google Research usz@google.com",
          metadata: {
            page: {
              number: 1,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 879.2488933479699,
              y: 489.58686880240765,
            },
            {
              x: 1035.003756780694,
              y: 489.58686880240765,
            },
            {
              x: 1035.003756780694,
              y: 555.848623204052,
            },
            {
              x: 879.2488933479699,
              y: 555.848623204052,
            },
          ],
          boundingBox: {
            left: 879.2488933479699,
            top: 489.58686880240765,
            right: 1035.003756780694,
            bottom: 555.848623204052,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_2_UD1C1k",
          type: "heading",
          content: "# 1 Introduction",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Heading",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 225.86025321570625,
              y: 152.1792611932396,
            },
            {
              x: 396.86960095036636,
              y: 152.1792611932396,
            },
            {
              x: 396.86960095036636,
              y: 171.96800693712737,
            },
            {
              x: 225.86025321570625,
              y: 171.96800693712737,
            },
          ],
          boundingBox: {
            left: 225.86025321570625,
            top: 152.1792611932396,
            right: 396.86960095036636,
            bottom: 171.96800693712737,
          },
        },
        {
          id: "block_2_C25o9D",
          type: "text",
          content:
            "Recurrent neural networks, long short-term memory [13] and gated recurrent [7] neural networks in particular, have been firmly established as state of the art approaches in sequence modeling and transduction problems such as language modeling and machine translation [35, 2, 5]. Numerous efforts have since continued to push the boundaries of recurrent language models and encoder-decoder architectures [38, 24, 15].",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.908,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 223.64054213475134,
              y: 206.4059457563816,
            },
            {
              x: 1050.9159227357293,
              y: 206.4059457563816,
            },
            {
              x: 1050.9159227357293,
              y: 316.89823061720773,
            },
            {
              x: 223.64054213475134,
              y: 316.89823061720773,
            },
          ],
          boundingBox: {
            left: 223.64054213475134,
            top: 206.4059457563816,
            right: 1050.9159227357293,
            bottom: 316.89823061720773,
          },
        },
        {
          id: "block_2_Cw1VyU",
          type: "text",
          content:
            "Recurrent models typically factor computation along the symbol positions of the input and output sequences. Aligning the positions to steps in computation time, they generate a sequence of hidden states ht, as a function of the previous hidden state ht-1 and the input for position t. This inherently sequential nature precludes parallelization within training examples, which becomes critical at longer sequence lengths, as memory constraints limit batching across examples. Recent work has achieved significant improvements in computational efficiency through factorization tricks [21] and conditional computation [32], while also improving model performance in case of the latter. The fundamental constraint of sequential computation, however, remains.",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.9,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 223.69928568819142,
              y: 331.227654822787,
            },
            {
              x: 1049.6158961832089,
              y: 331.227654822787,
            },
            {
              x: 1049.6158961832089,
              y: 511.80734250061494,
            },
            {
              x: 223.69928568819142,
              y: 511.80734250061494,
            },
          ],
          boundingBox: {
            left: 223.69928568819142,
            top: 331.227654822787,
            right: 1049.6158961832089,
            bottom: 511.80734250061494,
          },
        },
        {
          id: "block_2_FR7zlH",
          type: "text",
          content:
            "Attention mechanisms have become an integral part of compelling sequence modeling and transduc- tion models in various tasks, allowing modeling of dependencies without regard to their distance in the input or output sequences [2, 19]. In all but a few cases [27], however, such attention mechanisms are used in conjunction with a recurrent network.",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.959,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 223.38852708357095,
              y: 525.5238181666324,
            },
            {
              x: 1052.1732525233804,
              y: 525.5238181666324,
            },
            {
              x: 1052.1732525233804,
              y: 613.7107948647406,
            },
            {
              x: 223.38852708357095,
              y: 613.7107948647406,
            },
          ],
          boundingBox: {
            left: 223.38852708357095,
            top: 525.5238181666324,
            right: 1052.1732525233804,
            bottom: 613.7107948647406,
          },
        },
        {
          id: "block_2_ZyhlFh",
          type: "text",
          content:
            "In this work we propose the Transformer, a model architecture eschewing recurrence and instead relying entirely on an attention mechanism to draw global dependencies between input and output. The Transformer allows for significantly more parallelization and can reach a new state of the art in translation quality after being trained for as little as twelve hours on eight P100 GPUs.",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.991,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 223.20226126343667,
              y: 627.2328828797304,
            },
            {
              x: 1052.7354108156078,
              y: 627.2328828797304,
            },
            {
              x: 1052.7354108156078,
              y: 716.0174654623619,
            },
            {
              x: 223.20226126343667,
              y: 716.0174654623619,
            },
          ],
          boundingBox: {
            left: 223.20226126343667,
            top: 627.2328828797304,
            right: 1052.7354108156078,
            bottom: 716.0174654623619,
          },
        },
        {
          id: "block_2_MZBXjP",
          type: "section_heading",
          content: "### 2 Background",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.994,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 224.22355178498873,
              y: 757.4257287333783,
            },
            {
              x: 392.45446198178036,
              y: 757.4257287333783,
            },
            {
              x: 392.45446198178036,
              y: 781.2853567654029,
            },
            {
              x: 224.22355178498873,
              y: 781.2853567654029,
            },
          ],
          boundingBox: {
            left: 224.22355178498873,
            top: 757.4257287333783,
            right: 392.45446198178036,
            bottom: 781.2853567654029,
          },
        },
        {
          id: "block_2_JeOnOk",
          type: "text",
          content:
            "The goal of reducing sequential computation also forms the foundation of the Extended Neural GPU [16], ByteNet [18] and ConvS2S [9], all of which use convolutional neural networks as basic building block, computing hidden representations in parallel for all input and output positions. In these models, the number of operations required to relate signals from two arbitrary input or output positions grows in the distance between positions, linearly for ConvS2S and logarithmically for ByteNet. This makes it more difficult to learn dependencies between distant positions [12]. In the Transformer this is reduced to a constant number of operations, albeit at the cost of reduced effective resolution due to averaging attention-weighted positions, an effect we counteract with Multi-Head Attention as described in section 3.2.",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.893,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 223.30221050847186,
              y: 810.9956032315591,
            },
            {
              x: 1051.187440078624,
              y: 810.9956032315591,
            },
            {
              x: 1051.187440078624,
              y: 1010.7095232511822,
            },
            {
              x: 223.30221050847186,
              y: 1010.7095232511822,
            },
          ],
          boundingBox: {
            left: 223.30221050847186,
            top: 810.9956032315591,
            right: 1051.187440078624,
            bottom: 1010.7095232511822,
          },
        },
        {
          id: "block_2_tEzlrD",
          type: "text",
          content:
            "Self-attention, sometimes called intra-attention is an attention mechanism relating different positions of a single sequence in order to compute a representation of the sequence. Self-attention has been used successfully in a variety of tasks including reading comprehension, abstractive summarization, textual entailment and learning task-independent sentence representations [4, 27, 28, 22].",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.984,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 224.00045882176306,
              y: 1028.0413251030714,
            },
            {
              x: 1051.4467448213675,
              y: 1028.0413251030714,
            },
            {
              x: 1051.4467448213675,
              y: 1116.9915041815966,
            },
            {
              x: 224.00045882176306,
              y: 1116.9915041815966,
            },
          ],
          boundingBox: {
            left: 224.00045882176306,
            top: 1028.0413251030714,
            right: 1051.4467448213675,
            bottom: 1116.9915041815966,
          },
        },
        {
          id: "block_2_AFJ4ch",
          type: "text",
          content:
            "End-to-end memory networks are based on a recurrent attention mechanism instead of sequence- aligned recurrence and have been shown to perform well on simple-language question answering and language modeling tasks [34].",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.963,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 224.05524984763485,
              y: 1129.9793457949072,
            },
            {
              x: 1052.8165441359918,
              y: 1129.9793457949072,
            },
            {
              x: 1052.8165441359918,
              y: 1196.1117767678168,
            },
            {
              x: 224.05524984763485,
              y: 1196.1117767678168,
            },
          ],
          boundingBox: {
            left: 224.05524984763485,
            top: 1129.9793457949072,
            right: 1052.8165441359918,
            bottom: 1196.1117767678168,
          },
        },
        {
          id: "block_2_CBbJP3",
          type: "text",
          content:
            "To the best of our knowledge, however, the Transformer is the first transduction model relying entirely on self-attention to compute representations of its input and output without using sequence- aligned RNNs or convolution. In the following sections, we will describe the Transformer, motivate self-attention and discuss its advantages over models such as [17, 18] and [9].",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.898,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 222.86987026242443,
              y: 1210.5673043387276,
            },
            {
              x: 1052.2374396776631,
              y: 1210.5673043387276,
            },
            {
              x: 1052.2374396776631,
              y: 1298.352294004053,
            },
            {
              x: 222.86987026242443,
              y: 1298.352294004053,
            },
          ],
          boundingBox: {
            left: 222.86987026242443,
            top: 1210.5673043387276,
            right: 1052.2374396776631,
            bottom: 1298.352294004053,
          },
        },
        {
          id: "block_2_RlRHBW",
          type: "section_heading",
          content: "### 3 Model Architecture",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.993,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 224.20267675914906,
              y: 1339.2827093368187,
            },
            {
              x: 470.2105250671832,
              y: 1339.2827093368187,
            },
            {
              x: 470.2105250671832,
              y: 1359.0320211424864,
            },
            {
              x: 224.20267675914906,
              y: 1359.0320211424864,
            },
          ],
          boundingBox: {
            left: 224.20267675914906,
            top: 1339.2827093368187,
            right: 470.2105250671832,
            bottom: 1359.0320211424864,
          },
        },
        {
          id: "block_2_OhVaOm",
          type: "text",
          content:
            "Most competitive neural sequence transduction models have an encoder-decoder structure [5, 2, 35]. Here, the encoder maps an input sequence of symbol representations (x1, ... , In) to a sequence of continuous representations z = (Z1, ... , Zn). Given z, the decoder then generates an output sequence (31, ... , ym) of symbols one element at a time. At each step the model is auto-regressive [10], consuming the previously generated symbols as additional input when generating the next.",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.619,
            avgOcrConfidence: 0.969,
          },
          polygon: [
            {
              x: 223.9033023806384,
              y: 1394.2295871247027,
            },
            {
              x: 1051.703398767179,
              y: 1394.2295871247027,
            },
            {
              x: 1051.703398767179,
              y: 1505.850527885265,
            },
            {
              x: 223.9033023806384,
              y: 1505.850527885265,
            },
          ],
          boundingBox: {
            left: 223.9033023806384,
            top: 1394.2295871247027,
            right: 1051.703398767179,
            bottom: 1505.850527885265,
          },
        },
        {
          id: "block_2_MzhUlf",
          type: "page_number",
          content: "2",
          metadata: {
            page: {
              number: 2,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.995,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 631.4816022441336,
              y: 1547.6769732138268,
            },
            {
              x: 642.0810031194757,
              y: 1547.6769732138268,
            },
            {
              x: 642.0810031194757,
              y: 1563.2578182363868,
            },
            {
              x: 631.4816022441336,
              y: 1563.2578182363868,
            },
          ],
          boundingBox: {
            left: 631.4816022441336,
            top: 1547.6769732138268,
            right: 642.0810031194757,
            bottom: 1563.2578182363868,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_3_afwbz8",
          type: "figure",
          content:
            '<figure type="diagram">\nOutput Probabilities\n\nSoftmax\n\nLinear\n\nAdd & Norm Feed Forward\n\nAdd & Norm\n\nAdd & Norm\n\nMulti-Head Feed Attention Forward Nx\n\nAdd & Norm\n\nNx Add & Norm\n\nMasked Multi-Head Multi-Head Attention Attention\n\nPositional Positional Encoding Encoding Input Output Embedding Embedding\n\nInputs Outputs (shifted right)\n<caption>Transformer Architecture Diagram\n\nLeft Side: Encoder (Nx)\n- Inputs\n\u2193\n- Input Embedding\n\u2193\n- Positional Encoding (added to Input Embedding)\n\u2193\n- Multi-Head Attention\n\u2193\n- Add & Norm\n\u2193\n- Feed Forward\n\u2193\n- Add & Norm\n\nRight Side: Decoder (Nx)\n- Outputs (shifted right)\n\u2193\n- Output Embedding\n\u2193\n- Positional Encoding (added to Output Embedding)\n\u2193\n- Masked Multi-Head Attention\n\u2193\n- Add & Norm\n\u2193\n- Multi-Head Attention (receives input from Encoder\'s Add & Norm)\n\u2193\n- Add & Norm\n\u2193\n- Feed Forward\n\u2193\n- Add & Norm\n\u2193\n- Linear\n\u2193\n- Softmax\n\u2193\n- Output Probabilities</caption>\n</figure>',
          metadata: {
            page: {
              number: 3,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Picture/Figure/Image/Chart",
            minOcrConfidence: 0.868,
            avgOcrConfidence: 0.987,
          },
          polygon: [
            {
              x: 409.83791490540887,
              y: 150.86351013900642,
            },
            {
              x: 863.1759754932709,
              y: 150.86351013900642,
            },
            {
              x: 863.1759754932709,
              y: 821.7723977798806,
            },
            {
              x: 409.83791490540887,
              y: 821.7723977798806,
            },
          ],
          boundingBox: {
            left: 409.83791490540887,
            top: 150.86351013900642,
            right: 863.1759754932709,
            bottom: 821.7723977798806,
          },
        },
        {
          id: "block_3_1WIk1S",
          type: "text",
          content: "Figure 1: The Transformer - model architecture.",
          metadata: {
            page: {
              number: 3,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.989,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 436.4703770101505,
              y: 843.2910397178248,
            },
            {
              x: 836.1937028648209,
              y: 843.2910397178248,
            },
            {
              x: 836.1937028648209,
              y: 863.7018851373429,
            },
            {
              x: 436.4703770101505,
              y: 863.7018851373429,
            },
          ],
          boundingBox: {
            left: 436.4703770101505,
            top: 843.2910397178248,
            right: 836.1937028648209,
            bottom: 863.7018851373429,
          },
        },
        {
          id: "block_3_jyPEOL",
          type: "text",
          content:
            "The Transformer follows this overall architecture using stacked self-attention and point-wise, fully connected layers for both the encoder and decoder, shown in the left and right halves of Figure 1, respectively.",
          metadata: {
            page: {
              number: 3,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.973,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 222.86804783953366,
              y: 910.1303693154701,
            },
            {
              x: 1051.9256870241932,
              y: 910.1303693154701,
            },
            {
              x: 1051.9256870241932,
              y: 977.3362178945899,
            },
            {
              x: 222.86804783953366,
              y: 977.3362178945899,
            },
          ],
          boundingBox: {
            left: 222.86804783953366,
            top: 910.1303693154701,
            right: 1051.9256870241932,
            bottom: 977.3362178945899,
          },
        },
        {
          id: "block_3_nEQfm1",
          type: "section_heading",
          content: "### 3.1 Encoder and Decoder Stacks",
          metadata: {
            page: {
              number: 3,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.995,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 223.72352154585568,
              y: 1007.3903008367782,
            },
            {
              x: 527.4041196725664,
              y: 1007.3903008367782,
            },
            {
              x: 527.4041196725664,
              y: 1023.9329030316575,
            },
            {
              x: 223.72352154585568,
              y: 1023.9329030316575,
            },
          ],
          boundingBox: {
            left: 223.72352154585568,
            top: 1007.3903008367782,
            right: 527.4041196725664,
            bottom: 1023.9329030316575,
          },
        },
        {
          id: "block_3_W2U9b2",
          type: "text",
          content:
            "Encoder: The encoder is composed of a stack of N = 6 identical layers. Each layer has two sub-layers. The first is a multi-head self-attention mechanism, and the second is a simple, position- wise fully connected feed-forward network. We employ a residual connection [11] around each of the two sub-layers, followed by layer normalization [1]. That is, the output of each sub-layer is LayerNorm(x + Sublayer(x)), where Sublayer(x) is the function implemented by the sub-layer itself. To facilitate these residual connections, all sub-layers in the model, as well as the embedding layers, produce outputs of dimension dmodel = 512.",
          metadata: {
            page: {
              number: 3,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.657,
            avgOcrConfidence: 0.984,
          },
          polygon: [
            {
              x: 223.8025010937322,
              y: 1048.091523421438,
            },
            {
              x: 1053.5889674277203,
              y: 1048.091523421438,
            },
            {
              x: 1053.5889674277203,
              y: 1206.3018568225373,
            },
            {
              x: 223.8025010937322,
              y: 1206.3018568225373,
            },
          ],
          boundingBox: {
            left: 223.8025010937322,
            top: 1048.091523421438,
            right: 1053.5889674277203,
            bottom: 1206.3018568225373,
          },
        },
        {
          id: "block_3_8wxg0F",
          type: "text",
          content:
            "Decoder: The decoder is also composed of a stack of N = 6 identical layers. In addition to the two sub-layers in each encoder layer, the decoder inserts a third sub-layer, which performs multi-head attention over the output of the encoder stack. Similar to the encoder, we employ residual connections around each of the sub-layers, followed by layer normalization. We also modify the self-attention sub-layer in the decoder stack to prevent positions from attending to subsequent positions. This masking, combined with fact that the output embeddings are offset by one position, ensures that the predictions for position i can depend only on the known outputs at positions less than i.",
          metadata: {
            page: {
              number: 3,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.633,
            avgOcrConfidence: 0.99,
          },
          polygon: [
            {
              x: 224.16407452882643,
              y: 1233.3413921729066,
            },
            {
              x: 1049.721170689938,
              y: 1233.3413921729066,
            },
            {
              x: 1049.721170689938,
              y: 1391.0596241198086,
            },
            {
              x: 224.16407452882643,
              y: 1391.0596241198086,
            },
          ],
          boundingBox: {
            left: 224.16407452882643,
            top: 1233.3413921729066,
            right: 1049.721170689938,
            bottom: 1391.0596241198086,
          },
        },
        {
          id: "block_3_i0stGS",
          type: "section_heading",
          content: "### 3.2 Attention",
          metadata: {
            page: {
              number: 3,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.995,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 224.0832962258889,
              y: 1421.7722100279386,
            },
            {
              x: 354.57085797386446,
              y: 1421.7722100279386,
            },
            {
              x: 354.57085797386446,
              y: 1437.668053017523,
            },
            {
              x: 224.0832962258889,
              y: 1437.668053017523,
            },
          ],
          boundingBox: {
            left: 224.0832962258889,
            top: 1421.7722100279386,
            right: 354.57085797386446,
            bottom: 1437.668053017523,
          },
        },
        {
          id: "block_3_LwJLS3",
          type: "text",
          content:
            "An attention function can be described as mapping a query and a set of key-value pairs to an output, where the query, keys, values, and output are all vectors. The output is computed as a weighted sum",
          metadata: {
            page: {
              number: 3,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.986,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 223.38670466068018,
              y: 1462.3713376611695,
            },
            {
              x: 1052.432178580848,
              y: 1462.3713376611695,
            },
            {
              x: 1052.432178580848,
              y: 1506.3117546138906,
            },
            {
              x: 223.38670466068018,
              y: 1506.3117546138906,
            },
          ],
          boundingBox: {
            left: 223.38670466068018,
            top: 1462.3713376611695,
            right: 1052.432178580848,
            bottom: 1506.3117546138906,
          },
        },
        {
          id: "block_3_gqT8MC",
          type: "page_number",
          content: "3",
          metadata: {
            page: {
              number: 3,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.997,
            avgOcrConfidence: 0.997,
          },
          polygon: [
            {
              x: 632.2706403523466,
              y: 1547.236487481827,
            },
            {
              x: 642.0801037419451,
              y: 1547.236487481827,
            },
            {
              x: 642.0801037419451,
              y: 1562.977578012567,
            },
            {
              x: 632.2706403523466,
              y: 1562.977578012567,
            },
          ],
          boundingBox: {
            left: 632.2706403523466,
            top: 1547.236487481827,
            right: 642.0801037419451,
            bottom: 1562.977578012567,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_4_PplY2k",
          type: "header",
          content: "Scaled Dot-Product Attention",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Header",
            minOcrConfidence: 0.993,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 307.9120037329458,
              y: 148.57123934236685,
            },
            {
              x: 554.1308270753735,
              y: 148.57123934236685,
            },
            {
              x: 554.1308270753735,
              y: 164.4924521697195,
            },
            {
              x: 307.9120037329458,
              y: 164.4924521697195,
            },
          ],
          boundingBox: {
            left: 307.9120037329458,
            top: 148.57123934236685,
            right: 554.1308270753735,
            bottom: 164.4924521697195,
          },
        },
        {
          id: "block_4_TZACGH",
          type: "text",
          content: "Multi-Head Attention",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.993,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 756.6973442578839,
              y: 148.30866211339045,
            },
            {
              x: 937.5759876557511,
              y: 148.30866211339045,
            },
            {
              x: 937.5759876557511,
              y: 164.44492071911804,
            },
            {
              x: 756.6973442578839,
              y: 164.44492071911804,
            },
          ],
          boundingBox: {
            left: 756.6973442578839,
            top: 148.30866211339045,
            right: 937.5759876557511,
            bottom: 164.44492071911804,
          },
        },
        {
          id: "block_4_yRQL9Y",
          type: "figure",
          content:
            '<figure type="diagram">\n1 MatMul\n\n1 SoftMax 1 Mask (opt.) 1 Scale 1 MatMul 1 1 Q K V\n<caption>Scaled Dot-Product Attention Diagram\n\nInputs: Q, K, V\n\n1. MatMul (Q, K)\n   \u2193\n2. Scale\n   \u2193\n3. Mask (opt.)\n   \u2193\n4. SoftMax\n   \u2193\n5. MatMul (Result of SoftMax, V)\n   \u2193\nOutput</caption>\n</figure>',
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Picture/Figure/Image/Chart",
            minOcrConfidence: 0.226,
            avgOcrConfidence: 0.815,
          },
          polygon: [
            {
              x: 364.7714322500855,
              y: 195.77549835434533,
            },
            {
              x: 497.28713766501767,
              y: 195.77549835434533,
            },
            {
              x: 497.28713766501767,
              y: 457.62115733784844,
            },
            {
              x: 364.7714322500855,
              y: 457.62115733784844,
            },
          ],
          boundingBox: {
            left: 364.7714322500855,
            top: 195.77549835434533,
            right: 497.28713766501767,
            bottom: 457.62115733784844,
          },
        },
        {
          id: "block_4_goCdts",
          type: "figure",
          content:
            '<figure type="diagram">\nLinear\n\nConcat\n\nScaled Dot-Product\n\nh\n\nAttention\n\nLinear Linear Linear\n\nV K Q\n<caption>Multi-Head Attention\n\nInputs: V, K, Q\n\u2193\nLinear (x3) \u2192 Scaled Dot-Product Attention (h heads)\n\u2193\nConcat\n\u2193\nLinear</caption>\n</figure>',
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Picture/Figure/Image/Chart",
            minOcrConfidence: 0.981,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 723.0441991430129,
              y: 171.2063080421964,
            },
            {
              x: 971.0292342805515,
              y: 171.2063080421964,
            },
            {
              x: 971.0292342805515,
              y: 495.8982404264292,
            },
            {
              x: 723.0441991430129,
              y: 495.8982404264292,
            },
          ],
          boundingBox: {
            left: 723.0441991430129,
            top: 171.2063080421964,
            right: 971.0292342805515,
            bottom: 495.8982404264292,
          },
        },
        {
          id: "block_4_ZQ6XHX",
          type: "text",
          content:
            "Figure 2: (left) Scaled Dot-Product Attention. (right) Multi-Head Attention consists of several attention layers running in parallel.",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.991,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 224.32437673972473,
              y: 572.2067347074809,
            },
            {
              x: 1050.060662039875,
              y: 572.2067347074809,
            },
            {
              x: 1050.060662039875,
              y: 615.706381804961,
            },
            {
              x: 224.32437673972473,
              y: 615.706381804961,
            },
          ],
          boundingBox: {
            left: 224.32437673972473,
            top: 572.2067347074809,
            right: 1050.060662039875,
            bottom: 615.706381804961,
          },
        },
        {
          id: "block_4_seZO33",
          type: "text",
          content:
            "of the values, where the weight assigned to each value is computed by a compatibility function of the query with the corresponding key.",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.989,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 224.44408862260138,
              y: 661.3754045586836,
            },
            {
              x: 1049.3653012018135,
              y: 661.3754045586836,
            },
            {
              x: 1049.3653012018135,
              y: 705.0281940833071,
            },
            {
              x: 224.44408862260138,
              y: 705.0281940833071,
            },
          ],
          boundingBox: {
            left: 224.44408862260138,
            top: 661.3754045586836,
            right: 1049.3653012018135,
            bottom: 705.0281940833071,
          },
        },
        {
          id: "block_4_CMgLHk",
          type: "section_heading",
          content: "### 3.2.1 Scaled Dot-Product Attention",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.53467593227862,
              y: 732.4767248397483,
            },
            {
              x: 549.2687448097842,
              y: 732.4767248397483,
            },
            {
              x: 549.2687448097842,
              y: 748.6887496431967,
            },
            {
              x: 223.53467593227862,
              y: 748.6887496431967,
            },
          ],
          boundingBox: {
            left: 223.53467593227862,
            top: 732.4767248397483,
            right: 549.2687448097842,
            bottom: 748.6887496431967,
          },
        },
        {
          id: "block_4_fmfLoy",
          type: "text",
          content:
            'We call our particular attention "Scaled Dot-Product Attention" (Figure 2). The input consists of queries and keys of dimension dk, and values of dimension dy. We compute the dot products of the\n\nquery with all keys, divide each by Vdk, and apply a softmafunction to obtain the weights on the [x]\n\nvalues.',
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.594,
            avgOcrConfidence: 0.98,
          },
          polygon: [
            {
              x: 223.39593511428276,
              y: 771.1101458269851,
            },
            {
              x: 1051.4275265436104,
              y: 771.1101458269851,
            },
            {
              x: 1051.4275265436104,
              y: 856.1295272138783,
            },
            {
              x: 223.39593511428276,
              y: 856.1295272138783,
            },
          ],
          boundingBox: {
            left: 223.39593511428276,
            top: 771.1101458269851,
            right: 1051.4275265436104,
            bottom: 856.1295272138783,
          },
        },
        {
          id: "block_4_cDuW6h",
          type: "text",
          content:
            "In practice, we compute the attention function on a set of queries simultaneously, packed together into a matrix Q. The keys and values are also packed together into matrices K and V. We compute the matrix of outputs as:",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.976,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 224.08788778486044,
              y: 873.4355685585424,
            },
            {
              x: 1050.3812191260122,
              y: 873.4355685585424,
            },
            {
              x: 1050.3812191260122,
              y: 938.6725377821385,
            },
            {
              x: 224.08788778486044,
              y: 938.6725377821385,
            },
          ],
          boundingBox: {
            left: 224.08788778486044,
            top: 873.4355685585424,
            right: 1050.3812191260122,
            bottom: 938.6725377821385,
          },
        },
        {
          id: "block_4_VaSWNE",
          type: "text",
          content: "QKT Attention(Q, K, V) = softmax( V Vdk",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Formula",
            minOcrConfidence: 0.908,
            avgOcrConfidence: 0.961,
          },
          polygon: [
            {
              x: 458.52888900868214,
              y: 968.9788274549901,
            },
            {
              x: 816.6205023327013,
              y: 968.9788274549901,
            },
            {
              x: 816.6205023327013,
              y: 1021.9395890773686,
            },
            {
              x: 458.52888900868214,
              y: 1021.9395890773686,
            },
          ],
          boundingBox: {
            left: 458.52888900868214,
            top: 968.9788274549901,
            right: 816.6205023327013,
            bottom: 1021.9395890773686,
          },
        },
        {
          id: "block_4_Wtl7xI",
          type: "text",
          content: "(1)",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.995,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 1026.9459968065694,
              y: 986.4570474839747,
            },
            {
              x: 1051.0514920645387,
              y: 986.4570474839747,
            },
            {
              x: 1051.0514920645387,
              y: 1005.8566982871607,
            },
            {
              x: 1026.9459968065694,
              y: 1005.8566982871607,
            },
          ],
          boundingBox: {
            left: 1026.9459968065694,
            top: 986.4570474839747,
            right: 1051.0514920645387,
            bottom: 1005.8566982871607,
          },
        },
        {
          id: "block_4_HVInAp",
          type: "text",
          content:
            "The two most commonly used attention functions are additive attention [2], and dot-product (multi- plicative) attention. Dot-product attention is identical to our algorithm, except for the scaling factor\n\nof . Additive attention computes the compatibility function using a feed-forward network with [x] Vdk\n\na single hidden layer. While the two are similar in theoretical complexity, dot-product attention is much faster and more space-efficient in practice, since it can be implemented using highly optimized matrix multiplication code.",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.098,
            avgOcrConfidence: 0.981,
          },
          polygon: [
            {
              x: 223.11696240501684,
              y: 1041.6009173859331,
            },
            {
              x: 1052.8067929901345,
              y: 1041.6009173859331,
            },
            {
              x: 1052.8067929901345,
              y: 1180.0422397699572,
            },
            {
              x: 223.11696240501684,
              y: 1180.0422397699572,
            },
          ],
          boundingBox: {
            left: 223.11696240501684,
            top: 1041.6009173859331,
            right: 1052.8067929901345,
            bottom: 1180.0422397699572,
          },
        },
        {
          id: "block_4_6NHlzf",
          type: "text",
          content:
            "While for small values of dk the two mechanisms perform similarly, additive attention outperforms dot product attention without scaling for larger values of dk [3]. We suspect that for large values of dk, the dot products grow large in magnitude, pushing the softmax function into regions where it has extremely small gradients 4. To counteract this effect, we scale the dot products by.",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.58,
            avgOcrConfidence: 0.98,
          },
          polygon: [
            {
              x: 223.18699551324775,
              y: 1194.0708248167111,
            },
            {
              x: 1050.8364934990875,
              y: 1194.0708248167111,
            },
            {
              x: 1050.8364934990875,
              y: 1289.5664457163416,
            },
            {
              x: 223.18699551324775,
              y: 1289.5664457163416,
            },
          ],
          boundingBox: {
            left: 223.18699551324775,
            top: 1194.0708248167111,
            right: 1050.8364934990875,
            bottom: 1289.5664457163416,
          },
        },
        {
          id: "block_4_4VWmvI",
          type: "section_heading",
          content: "### 3.2.2 Multi-Head Attention",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.994,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.5976323594142,
              y: 1315.4335938647278,
            },
            {
              x: 479.8474026422431,
              y: 1315.4335938647278,
            },
            {
              x: 479.8474026422431,
              y: 1331.4542616650574,
            },
            {
              x: 223.5976323594142,
              y: 1331.4542616650574,
            },
          ],
          boundingBox: {
            left: 223.5976323594142,
            top: 1315.4335938647278,
            right: 479.8474026422431,
            bottom: 1331.4542616650574,
          },
        },
        {
          id: "block_4_n7aPpb",
          type: "text",
          content:
            "Instead of performing a single attention function with dmodel-dimensional keys, values and queries, we found it beneficial to linearly project the queries, keys and values h times with different, learned linear projections to dk, dk and dy dimensions, respectively. On each of these projected versions of queries, keys and values we then perform the attention function in parallel, yielding dy-dimensional",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.619,
            avgOcrConfidence: 0.979,
          },
          polygon: [
            {
              x: 223.83094982509195,
              y: 1354.33461159154,
            },
            {
              x: 1052.0098498267848,
              y: 1354.33461159154,
            },
            {
              x: 1052.0098498267848,
              y: 1442.8801044664883,
            },
            {
              x: 223.83094982509195,
              y: 1442.8801044664883,
            },
          ],
          boundingBox: {
            left: 223.83094982509195,
            top: 1354.33461159154,
            right: 1052.0098498267848,
            bottom: 1442.8801044664883,
          },
        },
        {
          id: "block_4_rfqNPC",
          type: "text",
          content:
            "4To illustrate why the dot products get large, assume that the components of q and k are independent random variables with mean 0 and variance 1. Then their dot product, q \u00b7 k = >i=1 qiki, has mean 0 and variance dk-",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.105,
            avgOcrConfidence: 0.921,
          },
          polygon: [
            {
              x: 224.42529636577967,
              y: 1459.9841341864794,
            },
            {
              x: 1049.4534401998033,
              y: 1459.9841341864794,
            },
            {
              x: 1049.4534401998033,
              y: 1506.815978659723,
            },
            {
              x: 224.42529636577967,
              y: 1506.815978659723,
            },
          ],
          boundingBox: {
            left: 224.42529636577967,
            top: 1459.9841341864794,
            right: 1049.4534401998033,
            bottom: 1506.815978659723,
          },
        },
        {
          id: "block_4_35BhDQ",
          type: "page_number",
          content: "4",
          metadata: {
            page: {
              number: 4,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.996,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 631.2688757903385,
              y: 1548.1330857241064,
            },
            {
              x: 642.0564832478544,
              y: 1548.1330857241064,
            },
            {
              x: 642.0564832478544,
              y: 1562.0727401962854,
            },
            {
              x: 631.2688757903385,
              y: 1562.0727401962854,
            },
          ],
          boundingBox: {
            left: 631.2688757903385,
            top: 1548.1330857241064,
            right: 642.0564832478544,
            bottom: 1562.0727401962854,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_5_Zc8VSW",
          type: "text",
          content:
            "output values. These are concatenated and once again projected, resulting in the final values, as depicted in Figure 2.",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.988,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 224.67903916853186,
              y: 155.68632458564932,
            },
            {
              x: 1049.4719957783275,
              y: 155.68632458564932,
            },
            {
              x: 1049.4719957783275,
              y: 199.30535079841326,
            },
            {
              x: 224.67903916853186,
              y: 199.30535079841326,
            },
          ],
          boundingBox: {
            left: 224.67903916853186,
            top: 155.68632458564932,
            right: 1049.4719957783275,
            bottom: 199.30535079841326,
          },
        },
        {
          id: "block_5_P72mUL",
          type: "text",
          content:
            "Multi-head attention allows the model to jointly attend to information from different representation subspaces at different positions. With a single attention head, averaging inhibits this.",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 224.58133836732293,
              y: 212.29537068990837,
            },
            {
              x: 1049.5524663994781,
              y: 212.29537068990837,
            },
            {
              x: 1049.5524663994781,
              y: 255.9039790504857,
            },
            {
              x: 224.58133836732293,
              y: 255.9039790504857,
            },
          ],
          boundingBox: {
            left: 224.58133836732293,
            top: 212.29537068990837,
            right: 1049.5524663994781,
            bottom: 255.9039790504857,
          },
        },
        {
          id: "block_5_zTpvNY",
          type: "text",
          content:
            "MultiHead(Q,K,V)=Concat(head1, ... , headh)W\u00ba where head; = Attention(Qw?, KWK,VW)",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Formula",
            minOcrConfidence: 0.581,
            avgOcrConfidence: 0.796,
          },
          polygon: [
            {
              x: 388.73733464818804,
              y: 302.5467395065422,
            },
            {
              x: 883.3280911410811,
              y: 302.5467395065422,
            },
            {
              x: 883.3280911410811,
              y: 364.6030674267532,
            },
            {
              x: 388.73733464818804,
              y: 364.6030674267532,
            },
          ],
          boundingBox: {
            left: 388.73733464818804,
            top: 302.5467395065422,
            right: 883.3280911410811,
            bottom: 364.6030674267532,
          },
        },
        {
          id: "block_5_cOSQOc",
          type: "text",
          content:
            "Where the projections are parameter matrices Wo E Rdmodel X dk , WK \u20ac Rdmodel X dk , WY \u20ac Rdmodel X dy and WO E Rhdy Xdmodel",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.252,
            avgOcrConfidence: 0.682,
          },
          polygon: [
            {
              x: 223.51567066498916,
              y: 424.5787134959285,
            },
            {
              x: 1047.1388211215499,
              y: 424.5787134959285,
            },
            {
              x: 1047.1388211215499,
              y: 472.1556711698833,
            },
            {
              x: 223.51567066498916,
              y: 472.1556711698833,
            },
          ],
          boundingBox: {
            left: 223.51567066498916,
            top: 424.5787134959285,
            right: 1047.1388211215499,
            bottom: 472.1556711698833,
          },
        },
        {
          id: "block_5_4oR3J2",
          type: "text",
          content:
            "In this work we employ h = 8 parallel attention layers, or heads. For each of these we use dk = dy = dmodel/h = 64. Due to the reduced dimension of each head, the total computational cost is similar to that of single-head attention with full dimensionality.",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.557,
            avgOcrConfidence: 0.98,
          },
          polygon: [
            {
              x: 223.83189653828197,
              y: 488.323372747665,
            },
            {
              x: 1049.9097559573877,
              y: 488.323372747665,
            },
            {
              x: 1049.9097559573877,
              y: 555.0531728357302,
            },
            {
              x: 223.83189653828197,
              y: 555.0531728357302,
            },
          ],
          boundingBox: {
            left: 223.83189653828197,
            top: 488.323372747665,
            right: 1049.9097559573877,
            bottom: 555.0531728357302,
          },
        },
        {
          id: "block_5_oGR7Tp",
          type: "section_heading",
          content: "### 3.2.3 Applications of Attention in our Model",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.994,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 223.9031130380004,
              y: 584.1135347409356,
            },
            {
              x: 629.6546824657133,
              y: 584.1135347409356,
            },
            {
              x: 629.6546824657133,
              y: 605.02614180486,
            },
            {
              x: 223.9031130380004,
              y: 605.02614180486,
            },
          ],
          boundingBox: {
            left: 223.9031130380004,
            top: 584.1135347409356,
            right: 629.6546824657133,
            bottom: 605.02614180486,
          },
        },
        {
          id: "block_5_b34egv",
          type: "text",
          content:
            "The Transformer uses multi-head attention in three different ways:",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 223.3985859112148,
              y: 623.9660812033746,
            },
            {
              x: 775.7513199409428,
              y: 623.9660812033746,
            },
            {
              x: 775.7513199409428,
              y: 644.7702005429376,
            },
            {
              x: 223.3985859112148,
              y: 644.7702005429376,
            },
          ],
          boundingBox: {
            left: 223.3985859112148,
            top: 623.9660812033746,
            right: 775.7513199409428,
            bottom: 644.7702005429376,
          },
        },
        {
          id: "block_5_NSYaLO",
          type: "text",
          content:
            '\u00b7 In "encoder-decoder attention" layers, the queries come from the previous decoder layer, and the memory keys and values come from the output of the encoder. This allows every position in the decoder to attend over all positions in the input sequence. This mimics the typical encoder-decoder attention mechanisms in sequence-to-sequence models such as [38, 2, 9].',
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.586,
            avgOcrConfidence: 0.984,
          },
          polygon: [
            {
              x: 281.7719981618171,
              y: 667.4162908568419,
            },
            {
              x: 1052.1352893244612,
              y: 667.4162908568419,
            },
            {
              x: 1052.1352893244612,
              y: 778.3123385207098,
            },
            {
              x: 281.7719981618171,
              y: 778.3123385207098,
            },
          ],
          boundingBox: {
            left: 281.7719981618171,
            top: 667.4162908568419,
            right: 1052.1352893244612,
            bottom: 778.3123385207098,
          },
        },
        {
          id: "block_5_ZbEUX6",
          type: "text",
          content:
            "\u00b7 The encoder contains self-attention layers. In a self-attention layer all of the keys, values and queries come from the same place, in this case, the output of the previous layer in the encoder. Each position in the encoder can attend to all positions in the previous layer of the encoder.",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.407,
            avgOcrConfidence: 0.983,
          },
          polygon: [
            {
              x: 282.1731915439132,
              y: 791.5420637202442,
            },
            {
              x: 1049.6795153095775,
              y: 791.5420637202442,
            },
            {
              x: 1049.6795153095775,
              y: 876.9418387735697,
            },
            {
              x: 282.1731915439132,
              y: 876.9418387735697,
            },
          ],
          boundingBox: {
            left: 282.1731915439132,
            top: 791.5420637202442,
            right: 1049.6795153095775,
            bottom: 876.9418387735697,
          },
        },
        {
          id: "block_5_wkxEH8",
          type: "text",
          content:
            "\u00b7 Similarly, self-attention layers in the decoder allow each position in the decoder to attend to all positions in the decoder up to and including that position. We need to prevent leftward information flow in the decoder to preserve the auto-regressive property. We implement this inside of scaled dot-product attention by masking out (setting to -oo) all values in the input of the softmax which correspond to illegal connections. See Figure 2.",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.543,
            avgOcrConfidence: 0.985,
          },
          polygon: [
            {
              x: 281.12820952478114,
              y: 893.687589086088,
            },
            {
              x: 1049.8325041610829,
              y: 893.687589086088,
            },
            {
              x: 1049.8325041610829,
              y: 1005.0602061336201,
            },
            {
              x: 281.12820952478114,
              y: 1005.0602061336201,
            },
          ],
          boundingBox: {
            left: 281.12820952478114,
            top: 893.687589086088,
            right: 1049.8325041610829,
            bottom: 1005.0602061336201,
          },
        },
        {
          id: "block_5_vVMF0e",
          type: "section_heading",
          content: "### 3.3 Position-wise Feed-Forward Networks",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.59919443617773,
              y: 1037.180243872162,
            },
            {
              x: 609.3837069768975,
              y: 1037.180243872162,
            },
            {
              x: 609.3837069768975,
              y: 1053.935843790384,
            },
            {
              x: 223.59919443617773,
              y: 1053.935843790384,
            },
          ],
          boundingBox: {
            left: 223.59919443617773,
            top: 1037.180243872162,
            right: 609.3837069768975,
            bottom: 1053.935843790384,
          },
        },
        {
          id: "block_5_WtW3Xc",
          type: "text",
          content:
            "In addition to attention sub-layers, each of the layers in our encoder and decoder contains a fully connected feed-forward network, which is applied to each position separately and identically. This consists of two linear transformations with a ReLU activation in between.",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.986,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.937928415563,
              y: 1079.1571798109471,
            },
            {
              x: 1049.736412772297,
              y: 1079.1571798109471,
            },
            {
              x: 1049.736412772297,
              y: 1142.9025967246607,
            },
            {
              x: 223.937928415563,
              y: 1142.9025967246607,
            },
          ],
          boundingBox: {
            left: 223.937928415563,
            top: 1079.1571798109471,
            right: 1049.736412772297,
            bottom: 1142.9025967246607,
          },
        },
        {
          id: "block_5_TCB4QK",
          type: "text",
          content: "FFN(x)=max(0,xW1+b1)W2+b2",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Formula",
            minOcrConfidence: 0.668,
            avgOcrConfidence: 0.668,
          },
          polygon: [
            {
              x: 471.6477776965956,
              y: 1184.1576702146604,
            },
            {
              x: 801.1487473536581,
              y: 1184.1576702146604,
            },
            {
              x: 801.1487473536581,
              y: 1206.444581397494,
            },
            {
              x: 471.6477776965956,
              y: 1206.444581397494,
            },
          ],
          boundingBox: {
            left: 471.6477776965956,
            top: 1184.1576702146604,
            right: 801.1487473536581,
            bottom: 1206.444581397494,
          },
        },
        {
          id: "block_5_olGJjx",
          type: "text",
          content: "(2)",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.997,
            avgOcrConfidence: 0.997,
          },
          polygon: [
            {
              x: 1026.9936164800267,
              y: 1185.5856736262042,
            },
            {
              x: 1051.1806237436558,
              y: 1185.5856736262042,
            },
            {
              x: 1051.1806237436558,
              y: 1204.3962422421102,
            },
            {
              x: 1026.9936164800267,
              y: 1204.3962422421102,
            },
          ],
          boundingBox: {
            left: 1026.9936164800267,
            top: 1185.5856736262042,
            right: 1051.1806237436558,
            bottom: 1204.3962422421102,
          },
        },
        {
          id: "block_5_KTjDsL",
          type: "text",
          content:
            "While the linear transformations are the same across different positions, they use different parameters from layer to layer. Another way of describing this is as two convolutions with kernel size 1. The dimensionality of input and output is dmodel = 512, and the inner-layer has dimensionality dff =2048.",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.881,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 222.78797957148865,
              y: 1230.1093008858818,
            },
            {
              x: 1052.541145269018,
              y: 1230.1093008858818,
            },
            {
              x: 1052.541145269018,
              y: 1320.3967533541802,
            },
            {
              x: 222.78797957148865,
              y: 1320.3967533541802,
            },
          ],
          boundingBox: {
            left: 222.78797957148865,
            top: 1230.1093008858818,
            right: 1052.541145269018,
            bottom: 1320.3967533541802,
          },
        },
        {
          id: "block_5_Pwzvu2",
          type: "section_heading",
          content: "### 3.4 Embeddings and Softmax",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.995,
            avgOcrConfidence: 0.997,
          },
          polygon: [
            {
              x: 223.80512822283447,
              y: 1351.0096120045598,
            },
            {
              x: 499.655388045485,
              y: 1351.0096120045598,
            },
            {
              x: 499.655388045485,
              y: 1371.2419333206979,
            },
            {
              x: 223.80512822283447,
              y: 1371.2419333206979,
            },
          ],
          boundingBox: {
            left: 223.80512822283447,
            top: 1351.0096120045598,
            right: 499.655388045485,
            bottom: 1371.2419333206979,
          },
        },
        {
          id: "block_5_6y9ZGB",
          type: "text",
          content:
            "Similarly to other sequence transduction models, we use learned embeddings to convert the input tokens and output tokens to vectors of dimension dmodel. We also use the usual learned linear transfor- mation and softmax function to convert the decoder output to predicted next-token probabilities. In our model, we share the same weight matrix between the two embedding layers and the pre-softmax linear transformation, similar to [30]. In the embedding layers, we multiply those weights by Vdmodel -",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.275,
            avgOcrConfidence: 0.983,
          },
          polygon: [
            {
              x: 224.49331770848184,
              y: 1394.318801822519,
            },
            {
              x: 1052.2855327077154,
              y: 1394.318801822519,
            },
            {
              x: 1052.2855327077154,
              y: 1505.8341434450076,
            },
            {
              x: 224.49331770848184,
              y: 1505.8341434450076,
            },
          ],
          boundingBox: {
            left: 224.49331770848184,
            top: 1394.318801822519,
            right: 1052.2855327077154,
            bottom: 1505.8341434450076,
          },
        },
        {
          id: "block_5_o8MKrc",
          type: "page_number",
          content: "5",
          metadata: {
            page: {
              number: 5,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.995,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 631.5606054598397,
              y: 1547.4356578740858,
            },
            {
              x: 641.9868998283888,
              y: 1547.4356578740858,
            },
            {
              x: 641.9868998283888,
              y: 1563.1716341864794,
            },
            {
              x: 631.5606054598397,
              y: 1563.1716341864794,
            },
          ],
          boundingBox: {
            left: 631.5606054598397,
            top: 1547.4356578740858,
            right: 641.9868998283888,
            bottom: 1563.1716341864794,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_6_YmUPFZ",
          type: "text",
          content:
            "Table 1: Maximum path lengths, per-layer complexity and minimum number of sequential operations for different layer types. n is the sequence length, d is the representation dimension, k is the kernel size of convolutions and r the size of the neighborhood in restricted self-attention.",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.977,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 222.7798615058843,
              y: 148.2273318355245,
            },
            {
              x: 1048.906618661254,
              y: 148.2273318355245,
            },
            {
              x: 1048.906618661254,
              y: 214.40370720311216,
            },
            {
              x: 222.7798615058843,
              y: 214.40370720311216,
            },
          ],
          boundingBox: {
            left: 222.7798615058843,
            top: 148.2273318355245,
            right: 1048.906618661254,
            bottom: 214.40370720311216,
          },
        },
        {
          id: "block_6_kibLxu",
          type: "table",
          content:
            "| Layer Type | Complexity per Layer | Sequential Operations | Maximum Path Length |\n| --- | --- | --- | --- |\n| Self-Attention | O(n2 \u00b7 d) | O(1) | O(1) |\n| Recurrent | O(n \u00b7 d2) | O(n) | O(n) |\n| Convolutional | O(k \u00b7 n \u00b7 d2) | O(1) | O(logk(n)) |\n| Self-Attention (restricted) | O(r . n . d) [ ] | O(1) | O(n/r) [ ] |",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Table",
            minOcrConfidence: 0.256,
            avgOcrConfidence: 0.856,
          },
          polygon: [
            {
              x: 246.08119491243013,
              y: 233.8958637983279,
            },
            {
              x: 1028.2713006012632,
              y: 233.8958637983279,
            },
            {
              x: 1028.2713006012632,
              y: 389.94297754674926,
            },
            {
              x: 246.08119491243013,
              y: 389.94297754674926,
            },
          ],
          boundingBox: {
            left: 246.08119491243013,
            top: 233.8958637983279,
            right: 1028.2713006012632,
            bottom: 389.94297754674926,
          },
        },
        {
          id: "block_6_tUIpoY",
          type: "section_heading",
          content: "### 3.5 Positional Encoding",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 224.00651778617916,
              y: 445.9595083078944,
            },
            {
              x: 448.20427163674015,
              y: 445.9595083078944,
            },
            {
              x: 448.20427163674015,
              y: 466.69903696390026,
            },
            {
              x: 224.00651778617916,
              y: 466.69903696390026,
            },
          ],
          boundingBox: {
            left: 224.00651778617916,
            top: 445.9595083078944,
            right: 448.20427163674015,
            bottom: 466.69903696390026,
          },
        },
        {
          id: "block_6_mmxGrc",
          type: "text",
          content:
            'Since our model contains no recurrence and no convolution, in order for the model to make use of the order of the sequence, we must inject some information about the relative or absolute position of the tokens in the sequence. To this end, we add "positional encodings" to the input embeddings at the bottoms of the encoder and decoder stacks. The positional encodings have the same dimension dmodel as the embeddings, so that the two can be summed. There are many choices of positional encodings, learned and fixed [9].',
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.912,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.83698512167825,
              y: 487.3354288832586,
            },
            {
              x: 1050.4121766473256,
              y: 487.3354288832586,
            },
            {
              x: 1050.4121766473256,
              y: 620.7750930786133,
            },
            {
              x: 223.83698512167825,
              y: 620.7750930786133,
            },
          ],
          boundingBox: {
            left: 223.83698512167825,
            top: 487.3354288832586,
            right: 1050.4121766473256,
            bottom: 620.7750930786133,
          },
        },
        {
          id: "block_6_NG2aN8",
          type: "text",
          content:
            "In this work, we use sine and cosine functions of different frequencies:",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 224.39521455416715,
              y: 635.0555533789154,
            },
            {
              x: 810.5588871197108,
              y: 635.0555533789154,
            },
            {
              x: 810.5588871197108,
              y: 655.9754529393705,
            },
            {
              x: 224.39521455416715,
              y: 655.9754529393705,
            },
          ],
          boundingBox: {
            left: 224.39521455416715,
            top: 635.0555533789154,
            right: 810.5588871197108,
            bottom: 655.9754529393705,
          },
        },
        {
          id: "block_6_ktOzAP",
          type: "text",
          content:
            "PE(pos,2i) = sin(pos/100002i/dmodel ) PE(pos,2i+1) = cos(pos/100002i/dmodel) )",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Formula",
            minOcrConfidence: 0.646,
            avgOcrConfidence: 0.856,
          },
          polygon: [
            {
              x: 470.62755222738224,
              y: 700.6432727440855,
            },
            {
              x: 804.2361684089159,
              y: 700.6432727440855,
            },
            {
              x: 804.2361684089159,
              y: 763.5065343469606,
            },
            {
              x: 470.62755222738224,
              y: 763.5065343469606,
            },
          ],
          boundingBox: {
            left: 470.62755222738224,
            top: 700.6432727440855,
            right: 804.2361684089159,
            bottom: 763.5065343469606,
          },
        },
        {
          id: "block_6_L0vHNu",
          type: "text",
          content:
            "where pos is the position and i is the dimension. That is, each dimension of the positional encoding corresponds to a sinusoid. The wavelengths form a geometric progression from 27 to 10000 . 27. We chose this function because we hypothesized it would allow the model to easily learn to attend by relative positions, since for any fixed offset k, PEpos+k can be represented as a linear function of PEpos.",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.291,
            avgOcrConfidence: 0.967,
          },
          polygon: [
            {
              x: 223.63699196028884,
              y: 787.3148307800293,
            },
            {
              x: 1050.8335586881985,
              y: 787.3148307800293,
            },
            {
              x: 1050.8335586881985,
              y: 900.202535012611,
            },
            {
              x: 223.63699196028884,
              y: 900.202535012611,
            },
          ],
          boundingBox: {
            left: 223.63699196028884,
            top: 787.3148307800293,
            right: 1050.8335586881985,
            bottom: 900.202535012611,
          },
        },
        {
          id: "block_6_Sai49H",
          type: "text",
          content:
            "We also experimented with using learned positional embeddings [9] instead, and found that the two versions produced nearly identical results (see Table 3 row (E)). We chose the sinusoidal version because it may allow the model to extrapolate to sequence lengths longer than the ones encountered during training.",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.868,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 223.15007369883736,
              y: 912.2991767252298,
            },
            {
              x: 1049.522644933993,
              y: 912.2991767252298,
            },
            {
              x: 1049.522644933993,
              y: 1000.5864015736975,
            },
            {
              x: 223.15007369883736,
              y: 1000.5864015736975,
            },
          ],
          boundingBox: {
            left: 223.15007369883736,
            top: 912.2991767252298,
            right: 1049.522644933993,
            bottom: 1000.5864015736975,
          },
        },
        {
          id: "block_6_545oKE",
          type: "section_heading",
          content: "### 4 Why Self-Attention",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 223.44270274586924,
              y: 1036.185907234823,
            },
            {
              x: 468.14631162768734,
              y: 1036.185907234823,
            },
            {
              x: 468.14631162768734,
              y: 1059.806398205291,
            },
            {
              x: 223.44270274586924,
              y: 1059.806398205291,
            },
          ],
          boundingBox: {
            left: 223.44270274586924,
            top: 1036.185907234823,
            right: 468.14631162768734,
            bottom: 1059.806398205291,
          },
        },
        {
          id: "block_6_rdkytl",
          type: "text",
          content:
            "In this section we compare various aspects of self-attention layers to the recurrent and convolu- tional layers commonly used for mapping one variable-length sequence of symbol representations (x1, ... , In) to another sequence of equal length (21, ... , Zn), with xi, Zi E Rd, such as a hidden layer in a typical sequence transduction encoder or decoder. Motivating our use of self-attention we consider three desiderata.",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.298,
            avgOcrConfidence: 0.936,
          },
          polygon: [
            {
              x: 222.55295802206888,
              y: 1087.3066867455505,
            },
            {
              x: 1052.7677884067061,
              y: 1087.3066867455505,
            },
            {
              x: 1052.7677884067061,
              y: 1195.3768446499244,
            },
            {
              x: 222.55295802206888,
              y: 1195.3768446499244,
            },
          ],
          boundingBox: {
            left: 222.55295802206888,
            top: 1087.3066867455505,
            right: 1052.7677884067061,
            bottom: 1195.3768446499244,
          },
        },
        {
          id: "block_6_y2UJTk",
          type: "text",
          content:
            "One is the total computational complexity per layer. Another is the amount of computation that can be parallelized, as measured by the minimum number of sequential operations required.",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 223.43593374656064,
              y: 1212.4905345601246,
            },
            {
              x: 1048.7435946499345,
              y: 1212.4905345601246,
            },
            {
              x: 1048.7435946499345,
              y: 1255.7911059730932,
            },
            {
              x: 223.43593374656064,
              y: 1255.7911059730932,
            },
          ],
          boundingBox: {
            left: 223.43593374656064,
            top: 1212.4905345601246,
            right: 1048.7435946499345,
            bottom: 1255.7911059730932,
          },
        },
        {
          id: "block_6_OGHshO",
          type: "text",
          content:
            "The third is the path length between long-range dependencies in the network. Learning long-range dependencies is a key challenge in many sequence transduction tasks. One key factor affecting the ability to learn such dependencies is the length of the paths forward and backward signals have to traverse in the network. The shorter these paths between any combination of positions in the input and output sequences, the easier it is to learn long-range dependencies [12]. Hence we also compare the maximum path length between any two input and output positions in networks composed of the different layer types.",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.983,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 222.87737296445525,
              y: 1268.8657945905413,
            },
            {
              x: 1049.749950770914,
              y: 1268.8657945905413,
            },
            {
              x: 1049.749950770914,
              y: 1426.1352065236945,
            },
            {
              x: 222.87737296445525,
              y: 1426.1352065236945,
            },
          ],
          boundingBox: {
            left: 222.87737296445525,
            top: 1268.8657945905413,
            right: 1049.749950770914,
            bottom: 1426.1352065236945,
          },
        },
        {
          id: "block_6_zgN3wO",
          type: "text",
          content:
            "As noted in Table 1, a self-attention layer connects all positions with a constant number of sequentially executed operations, whereas a recurrent layer requires O(n) sequential operations. In terms of computational complexity, self-attention layers are faster than recurrent layers when the sequence",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.963,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 223.4352473794979,
              y: 1439.235182109632,
            },
            {
              x: 1051.6017217705719,
              y: 1439.235182109632,
            },
            {
              x: 1051.6017217705719,
              y: 1506.1062388025728,
            },
            {
              x: 223.4352473794979,
              y: 1506.1062388025728,
            },
          ],
          boundingBox: {
            left: 223.4352473794979,
            top: 1439.235182109632,
            right: 1051.6017217705719,
            bottom: 1506.1062388025728,
          },
        },
        {
          id: "block_6_aNgpKT",
          type: "page_number",
          content: "6",
          metadata: {
            page: {
              number: 6,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.993,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 631.7702550957673,
              y: 1548.2886905526757,
            },
            {
              x: 642.7105200551722,
              y: 1548.2886905526757,
            },
            {
              x: 642.7105200551722,
              y: 1563.5418467700931,
            },
            {
              x: 631.7702550957673,
              y: 1563.5418467700931,
            },
          ],
          boundingBox: {
            left: 631.7702550957673,
            top: 1548.2886905526757,
            right: 642.7105200551722,
            bottom: 1563.5418467700931,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_7_u723hy",
          type: "text",
          content:
            "length n is smaller than the representation dimensionality d, which is most often the case with sentence representations used by state-of-the-art models in machine translations, such as word-piece [38] and byte-pair [31] representations. To improve computational performance for tasks involving very long sequences, self-attention could be restricted to considering only a neighborhood of size r in the input sequence centered around the respective output position. This would increase the maximum path length to O(n/r). We plan to investigate this approach further in future work.",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.909,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 223.89144479793353,
              y: 155.39061148363845,
            },
            {
              x: 1050.0338700565978,
              y: 155.39061148363845,
            },
            {
              x: 1050.0338700565978,
              y: 290.11272512163436,
            },
            {
              x: 223.89144479793353,
              y: 290.11272512163436,
            },
          ],
          boundingBox: {
            left: 223.89144479793353,
            top: 155.39061148363845,
            right: 1050.0338700565978,
            bottom: 290.11272512163436,
          },
        },
        {
          id: "block_7_SLgoxa",
          type: "text",
          content:
            "A single convolutional layer with kernel width k < n does not connect all pairs of input and output positions. Doing so requires a stack of O(n/k) convolutional layers in the case of contiguous kernels, or O(logk(n)) in the case of dilated convolutions [18], increasing the length of the longest paths between any two positions in the network. Convolutional layers are generally more expensive than recurrent layers, by a factor of k. Separable convolutions [6], however, decrease the complexity considerably, to O(k . n . d + n . d2). Even with k = n, however, the complexity of a separable convolution is equal to the combination of a self-attention layer and a point-wise feed-forward layer, the approach we take in our model.",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.525,
            avgOcrConfidence: 0.977,
          },
          polygon: [
            {
              x: 223.84822734080961,
              y: 303.4973448416344,
            },
            {
              x: 1051.7048188369638,
              y: 303.4973448416344,
            },
            {
              x: 1051.7048188369638,
              y: 482.7421505003047,
            },
            {
              x: 223.84822734080961,
              y: 482.7421505003047,
            },
          ],
          boundingBox: {
            left: 223.84822734080961,
            top: 303.4973448416344,
            right: 1051.7048188369638,
            bottom: 482.7421505003047,
          },
        },
        {
          id: "block_7_lGN5L0",
          type: "text",
          content:
            "As side benefit, self-attention could yield more interpretable models. We inspect attention distributions from our models and present and discuss examples in the appendix. Not only do individual attention heads clearly learn to perform different tasks, many appear to exhibit behavior related to the syntactic and semantic structure of the sentences.",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.94,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.7218884656029,
              y: 496.4417208334557,
            },
            {
              x: 1049.5183847246378,
              y: 496.4417208334557,
            },
            {
              x: 1049.5183847246378,
              y: 581.3287481437052,
            },
            {
              x: 223.7218884656029,
              y: 581.3287481437052,
            },
          ],
          boundingBox: {
            left: 223.7218884656029,
            top: 496.4417208334557,
            right: 1049.5183847246378,
            bottom: 581.3287481437052,
          },
        },
        {
          id: "block_7_FOtl5B",
          type: "heading",
          content: "# 5 Training",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Heading",
            minOcrConfidence: 0.989,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 224.4122553915873,
              y: 625.3390120599503,
            },
            {
              x: 354.91415984439146,
              y: 625.3390120599503,
            },
            {
              x: 354.91415984439146,
              y: 649.7166914688913,
            },
            {
              x: 224.4122553915873,
              y: 649.7166914688913,
            },
          ],
          boundingBox: {
            left: 224.4122553915873,
            top: 625.3390120599503,
            right: 354.91415984439146,
            bottom: 649.7166914688913,
          },
        },
        {
          id: "block_7_3jYBCf",
          type: "text",
          content: "This section describes the training regime for our models.",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 222.4527957665659,
              y: 679.1303132709704,
            },
            {
              x: 701.988161741382,
              y: 679.1303132709704,
            },
            {
              x: 701.988161741382,
              y: 699.7081758025894,
            },
            {
              x: 222.4527957665659,
              y: 699.7081758025894,
            },
          ],
          boundingBox: {
            left: 222.4527957665659,
            top: 679.1303132709704,
            right: 701.988161741382,
            bottom: 699.7081758025894,
          },
        },
        {
          id: "block_7_PIafds",
          type: "section_heading",
          content: "### 5.1 Training Data and Batching",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 223.68002007477475,
              y: 735.0401794318866,
            },
            {
              x: 519.6398407873446,
              y: 735.0401794318866,
            },
            {
              x: 519.6398407873446,
              y: 755.8565634247055,
            },
            {
              x: 223.68002007477475,
              y: 755.8565634247055,
            },
          ],
          boundingBox: {
            left: 223.68002007477475,
            top: 735.0401794318866,
            right: 519.6398407873446,
            bottom: 755.8565634247055,
          },
        },
        {
          id: "block_7_8RhsHH",
          type: "text",
          content:
            "We trained on the standard WMT 2014 English-German dataset consisting of about 4.5 million sentence pairs. Sentences were encoded using byte-pair encoding [3], which has a shared source- target vocabulary of about 37000 tokens. For English-French, we used the significantly larger WMT 2014 English-French dataset consisting of 36M sentences and split tokens into a 32000 word-piece vocabulary [38]. Sentence pairs were batched together by approximate sequence length. Each training batch contained a set of sentence pairs containing approximately 25000 source tokens and 25000 target tokens.",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.911,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.70165247116648,
              y: 778.8621643467953,
            },
            {
              x: 1052.1871692072737,
              y: 778.8621643467953,
            },
            {
              x: 1052.1871692072737,
              y: 935.23512009929,
            },
            {
              x: 223.70165247116648,
              y: 935.23512009929,
            },
          ],
          boundingBox: {
            left: 223.70165247116648,
            top: 778.8621643467953,
            right: 1052.1871692072737,
            bottom: 935.23512009929,
          },
        },
        {
          id: "block_7_2fOgoW",
          type: "section_heading",
          content: "### 5.2 Hardware and Schedule",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.994,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 223.9209349138023,
              y: 971.4598863071069,
            },
            {
              x: 484.7486314982394,
              y: 971.4598863071069,
            },
            {
              x: 484.7486314982394,
              y: 987.6652342143811,
            },
            {
              x: 223.9209349138023,
              y: 987.6652342143811,
            },
          ],
          boundingBox: {
            left: 223.9209349138023,
            top: 971.4598863071069,
            right: 484.7486314982394,
            bottom: 987.6652342143811,
          },
        },
        {
          id: "block_7_7vykWe",
          type: "text",
          content:
            "We trained our models on one machine with 8 NVIDIA P100 GPUs. For our base models using the hyperparameters described throughout the paper, each training step took about 0.4 seconds. We trained the base models for a total of 100,000 steps or 12 hours. For our big models,(described on the bottom line of table 3), step time was 1.0 seconds. The big models were trained for 300,000 steps (3.5 days).",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.961,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 223.2459757449853,
              y: 1014.9955223485043,
            },
            {
              x: 1049.3973947789548,
              y: 1014.9955223485043,
            },
            {
              x: 1049.3973947789548,
              y: 1126.7015106660083,
            },
            {
              x: 223.2459757449853,
              y: 1126.7015106660083,
            },
          ],
          boundingBox: {
            left: 223.2459757449853,
            top: 1014.9955223485043,
            right: 1049.3973947789548,
            bottom: 1126.7015106660083,
          },
        },
        {
          id: "block_7_Xjy6qb",
          type: "section_heading",
          content: "### 5.3 Optimizer",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.994,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.84361211400832,
              y: 1161.1210525053784,
            },
            {
              x: 361.9454223744191,
              y: 1161.1210525053784,
            },
            {
              x: 361.9454223744191,
              y: 1181.9203891037102,
            },
            {
              x: 223.84361211400832,
              y: 1181.9203891037102,
            },
          ],
          boundingBox: {
            left: 223.84361211400832,
            top: 1161.1210525053784,
            right: 361.9454223744191,
            bottom: 1181.9203891037102,
          },
        },
        {
          id: "block_7_uZS3FC",
          type: "text",
          content:
            "We used the Adam optimizer [20] with 31 = 0.9, 32 = 0.98 and \u20ac = 10-9. We varied the learning rate over the course of training, according to the formula:",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.472,
            avgOcrConfidence: 0.947,
          },
          polygon: [
            {
              x: 223.18536243299496,
              y: 1203.7906809068263,
            },
            {
              x: 1049.5936484232436,
              y: 1203.7906809068263,
            },
            {
              x: 1049.5936484232436,
              y: 1249.0720648514596,
            },
            {
              x: 223.18536243299496,
              y: 1249.0720648514596,
            },
          ],
          boundingBox: {
            left: 223.18536243299496,
            top: 1203.7906809068263,
            right: 1049.5936484232436,
            bottom: 1249.0720648514596,
          },
        },
        {
          id: "block_7_qvJczg",
          type: "text",
          content:
            "lrate = dmodel . min(step_num-0.5, step_num . warmup_steps-1.5)",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Formula",
            minOcrConfidence: 0.563,
            avgOcrConfidence: 0.786,
          },
          polygon: [
            {
              x: 338.51696905428474,
              y: 1286.921258338412,
            },
            {
              x: 933.735361586522,
              y: 1286.921258338412,
            },
            {
              x: 933.735361586522,
              y: 1314.0637410099346,
            },
            {
              x: 338.51696905428474,
              y: 1314.0637410099346,
            },
          ],
          boundingBox: {
            left: 338.51696905428474,
            top: 1286.921258338412,
            right: 933.735361586522,
            bottom: 1314.0637410099346,
          },
        },
        {
          id: "block_7_ZRnl1T",
          type: "text",
          content: "(3)",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 1026.6387883764114,
              y: 1291.0368781986094,
            },
            {
              x: 1051.2767151324417,
              y: 1291.0368781986094,
            },
            {
              x: 1051.2767151324417,
              y: 1311.080825805664,
            },
            {
              x: 1026.6387883764114,
              y: 1311.080825805664,
            },
          ],
          boundingBox: {
            left: 1026.6387883764114,
            top: 1291.0368781986094,
            right: 1051.2767151324417,
            bottom: 1311.080825805664,
          },
        },
        {
          id: "block_7_M71nL9",
          type: "text",
          content:
            "This corresponds to increasing the learning rate linearly for the first warmup_steps training steps, and decreasing it thereafter proportionally to the inverse square root of the step number. We used warmup_steps = 4000.",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.916,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 222.72840764400732,
              y: 1340.2761936044335,
            },
            {
              x: 1052.0266066502481,
              y: 1340.2761936044335,
            },
            {
              x: 1052.0266066502481,
              y: 1406.6737115616188,
            },
            {
              x: 222.72840764400732,
              y: 1406.6737115616188,
            },
          ],
          boundingBox: {
            left: 222.72840764400732,
            top: 1340.2761936044335,
            right: 1052.0266066502481,
            bottom: 1406.6737115616188,
          },
        },
        {
          id: "block_7_ACakMa",
          type: "section_heading",
          content: "### 5.4 Regularization",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.993,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 223.81116351942077,
              y: 1441.41800626597,
            },
            {
              x: 402.1996435457773,
              y: 1441.41800626597,
            },
            {
              x: 402.1996435457773,
              y: 1461.9926013803124,
            },
            {
              x: 223.81116351942077,
              y: 1461.9926013803124,
            },
          ],
          boundingBox: {
            left: 223.81116351942077,
            top: 1441.41800626597,
            right: 402.1996435457773,
            bottom: 1461.9926013803124,
          },
        },
        {
          id: "block_7_rR8JeL",
          type: "text",
          content: "We employ three types of regularization during training:",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 222.88627206844134,
              y: 1484.7892296583134,
            },
            {
              x: 690.987449144795,
              y: 1484.7892296583134,
            },
            {
              x: 690.987449144795,
              y: 1505.950728681751,
            },
            {
              x: 222.88627206844134,
              y: 1505.950728681751,
            },
          ],
          boundingBox: {
            left: 222.88627206844134,
            top: 1484.7892296583134,
            right: 690.987449144795,
            bottom: 1505.950728681751,
          },
        },
        {
          id: "block_7_I7j7Uy",
          type: "page_number",
          content: "7",
          metadata: {
            page: {
              number: 7,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.996,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 631.8865114755004,
              y: 1547.4189893105872,
            },
            {
              x: 642.4512153124288,
              y: 1547.4189893105872,
            },
            {
              x: 642.4512153124288,
              y: 1562.541448836936,
            },
            {
              x: 631.8865114755004,
              y: 1562.541448836936,
            },
          ],
          boundingBox: {
            left: 631.8865114755004,
            top: 1547.4189893105872,
            right: 642.4512153124288,
            bottom: 1562.541448836936,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_8_8rtbBp",
          type: "text",
          content:
            "Table 2: The Transformer achieves better BLEU scores than previous state-of-the-art models on the English-to-German and English-to-French newstest2014 tests at a fraction of the training cost.",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.963,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 222.40285664579295,
              y: 148.11078211418666,
            },
            {
              x: 1048.695596291201,
              y: 148.11078211418666,
            },
            {
              x: 1048.695596291201,
              y: 191.8824890538266,
            },
            {
              x: 222.40285664579295,
              y: 191.8824890538266,
            },
          ],
          boundingBox: {
            left: 222.40285664579295,
            top: 148.11078211418666,
            right: 1048.695596291201,
            bottom: 191.8824890538266,
          },
        },
        {
          id: "block_8_M3V7h9",
          type: "table",
          content:
            "| Model | BLEU |  | Training Cost (FLOPs) |  |\n| --- | --- | --- | --- | --- |\n|  | EN-DE | EN-FR | EN-DE | EN-FR |\n| ByteNet [18] | 23.75 |  |  |  |\n| Deep-Att + PosUnk [39] |  | 39.2 |  | 1.0 . 1020 |\n| GNMT + RL [38] | 24.6 | 39.92 | 2.3 . 1019 | 1.4 . 1020 |\n| ConvS2S [9] | 25.16 | 40.46 | 9.6 . 1018 | 1.5 . 1020 |\n| MoE [32] | 26.03 | 40.56 | 2.0 \u00b7 1019 | 1.2 . 1020 |\n| Deep-Att + PosUnk Ensemble [39] |  | 40.4 |  | 8.0 . 1020 |\n| GNMT + RL Ensemble [38] | 26.30 | 41.16 | 1.8 . 1020 | 1.1 . 1021 |\n| ConvS2S Ensemble [9] | 26.36 | 41.29 | 7.7 . 1019 | 1.2 . 1021 |\n| Transformer (base model) | 27.3 | 38.1 | 3.3 . | 1018 |\n| Transformer (big) | 28.4 | 41.8 | 2.3. 1019 |  |",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Table",
            minOcrConfidence: 0.141,
            avgOcrConfidence: 0.893,
          },
          polygon: [
            {
              x: 271.150136516042,
              y: 195.3426247575229,
            },
            {
              x: 1004.1704442379248,
              y: 195.3426247575229,
            },
            {
              x: 1004.1704442379248,
              y: 508.6132290201976,
            },
            {
              x: 271.150136516042,
              y: 508.6132290201976,
            },
          ],
          boundingBox: {
            left: 271.150136516042,
            top: 195.3426247575229,
            right: 1004.1704442379248,
            bottom: 508.6132290201976,
          },
        },
        {
          id: "block_8_Wl0xmI",
          type: "text",
          content:
            "Residual Dropout We apply dropout [33] to the output of each sub-layer, before it is added to the sub-layer input and normalized. In addition, we apply dropout to the sums of the embeddings and the positional encodings in both the encoder and decoder stacks. For the base model, we use a rate of Pdrop =0.1.",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.893,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 223.9333605244212,
              y: 569.1121117154458,
            },
            {
              x: 1051.605887308608,
              y: 569.1121117154458,
            },
            {
              x: 1051.605887308608,
              y: 660.0886577520155,
            },
            {
              x: 223.9333605244212,
              y: 660.0886577520155,
            },
          ],
          boundingBox: {
            left: 223.9333605244212,
            top: 569.1121117154458,
            right: 1051.605887308608,
            bottom: 660.0886577520155,
          },
        },
        {
          id: "block_8_CtqTnb",
          type: "text",
          content:
            "Label Smoothing During training, we employed label smoothing of value Els = 0.1 [36]. This hurts perplexity, as the model learns to be more unsure, but improves accuracy and BLEU score.",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.518,
            avgOcrConfidence: 0.977,
          },
          polygon: [
            {
              x: 223.52019122047145,
              y: 690.790115499855,
            },
            {
              x: 1049.6801780088103,
              y: 690.790115499855,
            },
            {
              x: 1049.6801780088103,
              y: 734.1354363233523,
            },
            {
              x: 223.52019122047145,
              y: 734.1354363233523,
            },
          ],
          boundingBox: {
            left: 223.52019122047145,
            top: 690.790115499855,
            right: 1049.6801780088103,
            bottom: 734.1354363233523,
          },
        },
        {
          id: "block_8_KA1kZT",
          type: "heading",
          content: "# 6 Results",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Heading",
            minOcrConfidence: 0.995,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 224.16400352533717,
              y: 774.585115131579,
            },
            {
              x: 339.42863018843383,
              y: 774.585115131579,
            },
            {
              x: 339.42863018843383,
              y: 793.809935433524,
            },
            {
              x: 224.16400352533717,
              y: 793.809935433524,
            },
          ],
          boundingBox: {
            left: 224.16400352533717,
            top: 774.585115131579,
            right: 339.42863018843383,
            bottom: 793.809935433524,
          },
        },
        {
          id: "block_8_LYkEL8",
          type: "section_heading",
          content: "### 6.1 Machine Translation",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.994,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 224.2968510537252,
              y: 828.3471990026029,
            },
            {
              x: 455.2866330111984,
              y: 828.3471990026029,
            },
            {
              x: 455.2866330111984,
              y: 844.3720339438072,
            },
            {
              x: 224.2968510537252,
              y: 844.3720339438072,
            },
          ],
          boundingBox: {
            left: 224.2968510537252,
            top: 828.3471990026029,
            right: 455.2866330111984,
            bottom: 844.3720339438072,
          },
        },
        {
          id: "block_8_UnI1Ea",
          type: "text",
          content:
            "On the WMT 2014 English-to-German translation task, the big transformer model (Transformer (big) in Table 2) outperforms the best previously reported models (including ensembles) by more than 2.0 BLEU, establishing a new state-of-the-art BLEU score of 28.4. The configuration of this model is listed in the bottom line of Table 3. Training took 3.5 days on 8 P100 GPUs. Even our base model surpasses all previously published models and ensembles, at a fraction of the training cost of any of the competitive models.",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.973,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.56291165317063,
              y: 872.0711140453367,
            },
            {
              x: 1050.5385628581919,
              y: 872.0711140453367,
            },
            {
              x: 1050.5385628581919,
              y: 1006.0390107004266,
            },
            {
              x: 223.56291165317063,
              y: 1006.0390107004266,
            },
          ],
          boundingBox: {
            left: 223.56291165317063,
            top: 872.0711140453367,
            right: 1050.5385628581919,
            bottom: 1006.0390107004266,
          },
        },
        {
          id: "block_8_pkBARc",
          type: "text",
          content:
            "On the WMT 2014 English-to-French translation task, our big model achieves a BLEU score of 41.0, outperforming all of the previously published single models, at less than 1/4 the training cost of the previous state-of-the-art model. The Transformer (big) model trained for English-to-French used dropout rate Pdrop = 0.1, instead of 0.3.",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.945,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 224.05139199138557,
              y: 1019.1086798072756,
            },
            {
              x: 1052.2523977460653,
              y: 1019.1086798072756,
            },
            {
              x: 1052.2523977460653,
              y: 1109.5784446888401,
            },
            {
              x: 224.05139199138557,
              y: 1109.5784446888401,
            },
          ],
          boundingBox: {
            left: 224.05139199138557,
            top: 1019.1086798072756,
            right: 1052.2523977460653,
            bottom: 1109.5784446888401,
          },
        },
        {
          id: "block_8_vKs7Yg",
          type: "text",
          content:
            "For the base models, we used a single model obtained by averaging the last 5 checkpoints, which were written at 10-minute intervals. For the big models, we averaged the last 20 checkpoints. We used beam search with a beam size of 4 and length penalty & = 0.6 [38]. These hyperparameters were chosen after experimentation on the development set. We set the maximum output length during inference to input length + 50, but terminate early when possible [38].",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.099,
            avgOcrConfidence: 0.983,
          },
          polygon: [
            {
              x: 223.50279536560504,
              y: 1122.2645474197273,
            },
            {
              x: 1050.043905216412,
              y: 1122.2645474197273,
            },
            {
              x: 1050.043905216412,
              y: 1233.5787297872673,
            },
            {
              x: 223.50279536560504,
              y: 1233.5787297872673,
            },
          ],
          boundingBox: {
            left: 223.50279536560504,
            top: 1122.2645474197273,
            right: 1050.043905216412,
            bottom: 1233.5787297872673,
          },
        },
        {
          id: "block_8_42PSVl",
          type: "text",
          content:
            "Table 2 summarizes our results and compares our translation quality and training costs to other model architectures from the literature. We estimate the number of floating point operations used to train a model by multiplying the training time, the number of GPUs used, and an estimate of the sustained single-precision floating-point capacity of each GPU 5.",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.975,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 222.3879459130503,
              y: 1247.2785842436597,
            },
            {
              x: 1049.63284234931,
              y: 1247.2785842436597,
            },
            {
              x: 1049.63284234931,
              y: 1336.1913537620603,
            },
            {
              x: 222.3879459130503,
              y: 1336.1913537620603,
            },
          ],
          boundingBox: {
            left: 222.3879459130503,
            top: 1247.2785842436597,
            right: 1049.63284234931,
            bottom: 1336.1913537620603,
          },
        },
        {
          id: "block_8_pDx0nB",
          type: "section_heading",
          content: "### 6.2 Model Variations",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 224.05955739264942,
              y: 1370.9984397027727,
            },
            {
              x: 424.93188398597886,
              y: 1370.9984397027727,
            },
            {
              x: 424.93188398597886,
              y: 1387.4261614003576,
            },
            {
              x: 224.05955739264942,
              y: 1387.4261614003576,
            },
          ],
          boundingBox: {
            left: 224.05955739264942,
            top: 1370.9984397027727,
            right: 424.93188398597886,
            bottom: 1387.4261614003576,
          },
        },
        {
          id: "block_8_XCLvsN",
          type: "text",
          content:
            "To evaluate the importance of different components of the Transformer, we varied our base model in different ways, measuring the change in performance on English-to-German translation on the",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 222.6526705888066,
              y: 1413.835037862448,
            },
            {
              x: 1049.9819901737853,
              y: 1413.835037862448,
            },
            {
              x: 1049.9819901737853,
              y: 1458.0598621798638,
            },
            {
              x: 222.6526705888066,
              y: 1458.0598621798638,
            },
          ],
          boundingBox: {
            left: 222.6526705888066,
            top: 1413.835037862448,
            right: 1049.9819901737853,
            bottom: 1458.0598621798638,
          },
        },
        {
          id: "block_8_6e9VRc",
          type: "text",
          content:
            "5We used values of 2.8, 3.7, 6.0 and 9.5 TFLOPS for K80, K40, M40 and P100, respectively.",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.976,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 249.0640041601919,
              y: 1482.8447849015545,
            },
            {
              x: 944.2274051861172,
              y: 1482.8447849015545,
            },
            {
              x: 944.2274051861172,
              y: 1505.4603319670025,
            },
            {
              x: 249.0640041601919,
              y: 1505.4603319670025,
            },
          ],
          boundingBox: {
            left: 249.0640041601919,
            top: 1482.8447849015545,
            right: 944.2274051861172,
            bottom: 1505.4603319670025,
          },
        },
        {
          id: "block_8_HHxEh9",
          type: "page_number",
          content: "8",
          metadata: {
            page: {
              number: 8,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.996,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 631.7743259624843,
              y: 1547.954182789738,
            },
            {
              x: 642.6607229413778,
              y: 1547.954182789738,
            },
            {
              x: 642.6607229413778,
              y: 1563.2957013352473,
            },
            {
              x: 631.7743259624843,
              y: 1563.2957013352473,
            },
          ],
          boundingBox: {
            left: 631.7743259624843,
            top: 1547.954182789738,
            right: 642.6607229413778,
            bottom: 1563.2957013352473,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_9_2CG18E",
          type: "text",
          content:
            "Table 3: Variations on the Transformer architecture. Unlisted values are identical to those of the base model. All metrics are on the English-to-German translation development set, newstest2013. Listed perplexities are per-wordpiece, according to our byte-pair encoding, and should not be compared to per-word perplexities.",
          metadata: {
            page: {
              number: 9,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.99,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 222.6654748847015,
              y: 147.86792777355453,
            },
            {
              x: 1049.6700481776775,
              y: 147.86792777355453,
            },
            {
              x: 1049.6700481776775,
              y: 237.19010704442076,
            },
            {
              x: 222.6654748847015,
              y: 237.19010704442076,
            },
          ],
          boundingBox: {
            left: 222.6654748847015,
            top: 147.86792777355453,
            right: 1049.6700481776775,
            bottom: 237.19010704442076,
          },
        },
        {
          id: "block_9_0WToWH",
          type: "table",
          content:
            "|  | N | dmodel | dff | h | dk | dv | Pdrop | Els | train steps | PPL (dev) | BLEU (dev) | params \u00d7106 |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n| base | 6 | 512 | 2048 | 8 | 64 | 64 | 0.1 | 0.1 | 100K | 4.92 | 25.8 | 65 |\n| (A) |  |  |  | 1 | 512 | 512 |  |  |  | 5.29 | 24.9 |  |\n|  |  |  |  | 4 | 128 | 128 |  |  |  | 5.00 | 25.5 |  |\n|  |  |  |  | 16 | 32 | 32 |  |  |  | 4.91 | 25.8 |  |\n|  |  |  |  | 32 | 16 | 16 |  |  |  | 5.01 | 25.4 |  |\n| (B) |  |  |  |  | 16 |  |  |  |  | 5.16 | 25.1 | 58 |\n|  |  |  |  |  | 32 |  |  |  |  | 5.01 | 25.4 | 60 |\n| (C) | 2 |  |  |  |  |  |  |  |  | 6.11 | 23.7 | 36 |\n|  | 4 |  |  |  |  |  |  |  |  | 5.19 | 25.3 | 50 |\n|  | 8 |  |  |  |  |  |  |  |  | 4.88 | 25.5 | 80 |\n|  |  | 256 |  |  | 32 | 32 |  |  |  | 5.75 | 24.5 | 28 |\n|  |  | 1024 |  |  | 128 | 128 |  |  |  | 4.66 | 26.0 | 168 |\n|  |  |  | 1024 |  |  |  |  |  |  | 5.12 | 25.4 | 53 |\n|  |  |  | 4096 |  |  |  |  |  |  | 4.75 | 26.2 | 90 |\n| (D) |  |  |  |  |  |  | 0.0 |  |  | 5.77 | 24.6 |  |\n|  |  |  |  |  |  |  | 0.2 |  |  | 4.95 | 25.5 |  |\n|  |  |  |  |  |  |  |  | 0.0 |  | 4.67 | 25.3 |  |\n|  |  |  |  |  |  |  |  | 0.2 |  | 5.47 | 25.7 |  |\n| (E) | positional embedding |  |  |  |  | instead of | sinusoids |  |  | 4.92 | 25.7 |  |\n| big | 6 | 1024 | 4096 | 16 |  |  | 0.3 |  | 300K | 4.33 | 26.4 | 213 |",
          metadata: {
            page: {
              number: 9,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Table",
            minOcrConfidence: 0.612,
            avgOcrConfidence: 0.986,
          },
          polygon: [
            {
              x: 224.84461930546448,
              y: 268.65403318763674,
            },
            {
              x: 1060.6742636130673,
              y: 268.65403318763674,
            },
            {
              x: 1060.6742636130673,
              y: 801.2216690178204,
            },
            {
              x: 224.84461930546448,
              y: 801.2216690178204,
            },
          ],
          boundingBox: {
            left: 224.84461930546448,
            top: 268.65403318763674,
            right: 1060.6742636130673,
            bottom: 801.2216690178204,
          },
        },
        {
          id: "block_9_p8WB9c",
          type: "text",
          content:
            "development set, newstest2013. We used beam search as described in the previous section, but no checkpoint averaging. We present these results in Table 3.",
          metadata: {
            page: {
              number: 9,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.989,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 224.42882287241247,
              y: 862.6040329323675,
            },
            {
              x: 1049.5265264580719,
              y: 862.6040329323675,
            },
            {
              x: 1049.5265264580719,
              y: 905.6541023541213,
            },
            {
              x: 224.42882287241247,
              y: 905.6541023541213,
            },
          ],
          boundingBox: {
            left: 224.42882287241247,
            top: 862.6040329323675,
            right: 1049.5265264580719,
            bottom: 905.6541023541213,
          },
        },
        {
          id: "block_9_XvsIA2",
          type: "text",
          content:
            "In Table 3 rows (A), we vary the number of attention heads and the attention key and value dimensions, keeping the amount of computation constant, as described in Section 3.2.2. While single-head attention is 0.9 BLEU worse than the best setting, quality also drops off with too many heads.",
          metadata: {
            page: {
              number: 9,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.967,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 224.23600106343736,
              y: 918.8913094656807,
            },
            {
              x: 1052.034559041044,
              y: 918.8913094656807,
            },
            {
              x: 1052.034559041044,
              y: 986.1080494357232,
            },
            {
              x: 224.23600106343736,
              y: 986.1080494357232,
            },
          ],
          boundingBox: {
            left: 224.23600106343736,
            top: 918.8913094656807,
            right: 1052.034559041044,
            bottom: 986.1080494357232,
          },
        },
        {
          id: "block_9_lFHLbG",
          type: "text",
          content:
            "In Table 3 rows (B), we observe that reducing the attention key size dk hurts model quality. This suggests that determining compatibility is not easy and that a more sophisticated compatibility function than dot product may be beneficial. We further observe in rows (C) and (D) that, as expected, bigger models are better, and dropout is very helpful in avoiding over-fitting. In row (E) we replace our sinusoidal positional encoding with learned positional embeddings [9], and observe nearly identical results to the base model.",
          metadata: {
            page: {
              number: 9,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.881,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 224.0262804240206,
              y: 998.9746651326803,
            },
            {
              x: 1052.338643317675,
              y: 998.9746651326803,
            },
            {
              x: 1052.338643317675,
              y: 1130.1984048140678,
            },
            {
              x: 224.0262804240206,
              y: 1130.1984048140678,
            },
          ],
          boundingBox: {
            left: 224.0262804240206,
            top: 998.9746651326803,
            right: 1052.338643317675,
            bottom: 1130.1984048140678,
          },
        },
        {
          id: "block_9_nnE2Kn",
          type: "section_heading",
          content: "### 6.3 English Constituency Parsing",
          metadata: {
            page: {
              number: 9,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.994,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 224.00199723069687,
              y: 1168.0604785546325,
            },
            {
              x: 533.1579584274849,
              y: 1168.0604785546325,
            },
            {
              x: 533.1579584274849,
              y: 1188.5333575485345,
            },
            {
              x: 224.00199723069687,
              y: 1188.5333575485345,
            },
          ],
          boundingBox: {
            left: 224.00199723069687,
            top: 1168.0604785546325,
            right: 533.1579584274849,
            bottom: 1188.5333575485345,
          },
        },
        {
          id: "block_9_UvLm1J",
          type: "text",
          content:
            "To evaluate if the Transformer can generalize to other tasks we performed experiments on English constituency parsing. This task presents specific challenges: the output is subject to strong structural constraints and is significantly longer than the input. Furthermore, RNN sequence-to-sequence models have not been able to attain state-of-the-art results in small-data regimes [37].",
          metadata: {
            page: {
              number: 9,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.963,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.0787388599702,
              y: 1212.099580979885,
            },
            {
              x: 1049.9080518736457,
              y: 1212.099580979885,
            },
            {
              x: 1049.9080518736457,
              y: 1300.6617424183323,
            },
            {
              x: 223.0787388599702,
              y: 1300.6617424183323,
            },
          ],
          boundingBox: {
            left: 223.0787388599702,
            top: 1212.099580979885,
            right: 1049.9080518736457,
            bottom: 1300.6617424183323,
          },
        },
        {
          id: "block_9_nmNLPf",
          type: "text",
          content:
            "We trained a 4-layer transformer with dmodel = 1024 on the Wall Street Journal (WSJ) portion of the Penn Treebank [25], about 40K training sentences. We also trained it in a semi-supervised setting, using the larger high-confidence and BerkleyParser corpora from with approximately 17M sentences [37]. We used a vocabulary of 16K tokens for the WSJ only setting and a vocabulary of 32K tokens for the semi-supervised setting.",
          metadata: {
            page: {
              number: 9,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.913,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 223.08401678600453,
              y: 1314.1105266370273,
            },
            {
              x: 1051.1732393807738,
              y: 1314.1105266370273,
            },
            {
              x: 1051.1732393807738,
              y: 1425.9921031177494,
            },
            {
              x: 223.08401678600453,
              y: 1425.9921031177494,
            },
          ],
          boundingBox: {
            left: 223.08401678600453,
            top: 1314.1105266370273,
            right: 1051.1732393807738,
            bottom: 1425.9921031177494,
          },
        },
        {
          id: "block_9_kQhH3x",
          type: "text",
          content:
            "We performed only a small number of experiments to select the dropout, both attention and residual (section 5.4), learning rates and beam size on the Section 22 development set, all other parameters remained unchanged from the English-to-German base translation model. During inference, we",
          metadata: {
            page: {
              number: 9,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 223.1762739863709,
              y: 1439.0945411051127,
            },
            {
              x: 1050.7557388639798,
              y: 1439.0945411051127,
            },
            {
              x: 1050.7557388639798,
              y: 1505.8386894168711,
            },
            {
              x: 223.1762739863709,
              y: 1505.8386894168711,
            },
          ],
          boundingBox: {
            left: 223.1762739863709,
            top: 1439.0945411051127,
            right: 1050.7557388639798,
            bottom: 1505.8386894168711,
          },
        },
        {
          id: "block_9_C4QuNg",
          type: "page_number",
          content: "9",
          metadata: {
            page: {
              number: 9,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.993,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 631.6446262554531,
              y: 1547.2787271370566,
            },
            {
              x: 642.3550765879834,
              y: 1547.2787271370566,
            },
            {
              x: 642.3550765879834,
              y: 1562.906831325445,
            },
            {
              x: 631.6446262554531,
              y: 1562.906831325445,
            },
          ],
          boundingBox: {
            left: 631.6446262554531,
            top: 1547.2787271370566,
            right: 642.3550765879834,
            bottom: 1562.906831325445,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_10_Whfo1k",
          type: "text",
          content:
            "Table 4: The Transformer generalizes well to English constituency parsing (Results are on Section 23 of WSJ)",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.988,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 222.60232911492787,
              y: 147.913837353986,
            },
            {
              x: 1049.0453121435903,
              y: 147.913837353986,
            },
            {
              x: 1049.0453121435903,
              y: 190.50408882485297,
            },
            {
              x: 222.60232911492787,
              y: 190.50408882485297,
            },
          ],
          boundingBox: {
            left: 222.60232911492787,
            top: 147.913837353986,
            right: 1049.0453121435903,
            bottom: 190.50408882485297,
          },
        },
        {
          id: "block_10_Oq0boE",
          type: "table",
          content:
            "| Parser | Training | WSJ 23 F1 |\n| --- | --- | --- |\n| Vinyals & Kaiser el al. (2014) [37] | WSJ only, discriminative | 88.3 |\n| Petrov et al. (2006) [29] | WSJ only, discriminative | 90.4 |\n| Zhu et al. (2013) [40] | WSJ only, discriminative | 90.4 |\n| Dyer et al. (2016) [8] | WSJ only, discriminative | 91.7 |\n| Transformer (4 layers) | WSJ only, discriminative | 91.3 |\n| Zhu et al. (2013) [40] | semi-supervised | 91.3 |\n| Huang & Harper (2009) [14] | semi-supervised | 91.3 |\n| McClosky et al. (2006) [26] | semi-supervised | 92.1 |\n| Vinyals & Kaiser el al. (2014) [37] | semi-supervised | 92.1 |\n| Transformer (4 layers) | semi-supervised | 92.7 |\n| Luong et al. (2015) [23] | multi-task | 93.0 |\n| Dyer et al. (2016) [8] | generative | 93.3 |",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Table",
            minOcrConfidence: 0.926,
            avgOcrConfidence: 0.99,
          },
          polygon: [
            {
              x: 300.48226962124346,
              y: 193.81453893417702,
            },
            {
              x: 974.1206315312072,
              y: 193.81453893417702,
            },
            {
              x: 974.1206315312072,
              y: 493.68785631567016,
            },
            {
              x: 300.48226962124346,
              y: 493.68785631567016,
            },
          ],
          boundingBox: {
            left: 300.48226962124346,
            top: 193.81453893417702,
            right: 974.1206315312072,
            bottom: 493.68785631567016,
          },
        },
        {
          id: "block_10_wSUPzw",
          type: "text",
          content:
            "increased the maximum output length to input length + 300. We used a beam size of 21 and & = 0.3 for both WSJ only and the semi-supervised setting.",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.117,
            avgOcrConfidence: 0.964,
          },
          polygon: [
            {
              x: 223.84354111051908,
              y: 546.3992524398001,
            },
            {
              x: 1049.2353174808252,
              y: 546.3992524398001,
            },
            {
              x: 1049.2353174808252,
              y: 589.787902057619,
            },
            {
              x: 223.84354111051908,
              y: 589.787902057619,
            },
          ],
          boundingBox: {
            left: 223.84354111051908,
            top: 546.3992524398001,
            right: 1049.2353174808252,
            bottom: 589.787902057619,
          },
        },
        {
          id: "block_10_KdYXbX",
          type: "text",
          content:
            "Our results in Table 4 show that despite the lack of task-specific tuning our model performs sur- prisingly well, yielding better results than all previously reported models with the exception of the Recurrent Neural Network Grammar [8].",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.917,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 224.34470740548016,
              y: 603.5821801379211,
            },
            {
              x: 1052.1757139776746,
              y: 603.5821801379211,
            },
            {
              x: 1052.1757139776746,
              y: 667.7562916691143,
            },
            {
              x: 224.34470740548016,
              y: 667.7562916691143,
            },
          ],
          boundingBox: {
            left: 224.34470740548016,
            top: 603.5821801379211,
            right: 1052.1757139776746,
            bottom: 667.7562916691143,
          },
        },
        {
          id: "block_10_spL6f1",
          type: "text",
          content:
            "In contrast to RNN sequence-to-sequence models [37], the Transformer outperforms the Berkeley- Parser [29] even when training only on the WSJ training set of 40K sentences.",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.907,
            avgOcrConfidence: 0.99,
          },
          polygon: [
            {
              x: 223.93137242672216,
              y: 683.4819921909419,
            },
            {
              x: 1052.021589070341,
              y: 683.4819921909419,
            },
            {
              x: 1052.021589070341,
              y: 725.8283830513632,
            },
            {
              x: 223.93137242672216,
              y: 725.8283830513632,
            },
          ],
          boundingBox: {
            left: 223.93137242672216,
            top: 683.4819921909419,
            right: 1052.021589070341,
            bottom: 725.8283830513632,
          },
        },
        {
          id: "block_10_FpxS4M",
          type: "section_heading",
          content: "### 7 Conclusion",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.993,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 223.87968188654767,
              y: 760.911873552136,
            },
            {
              x: 380.37754949862074,
              y: 760.911873552136,
            },
            {
              x: 380.37754949862074,
              y: 780.8896204439321,
            },
            {
              x: 223.87968188654767,
              y: 780.8896204439321,
            },
          ],
          boundingBox: {
            left: 223.87968188654767,
            top: 760.911873552136,
            right: 380.37754949862074,
            bottom: 780.8896204439321,
          },
        },
        {
          id: "block_10_06Cxhy",
          type: "text",
          content:
            "In this work, we presented the Transformer, the first sequence transduction model based entirely on attention, replacing the recurrent layers most commonly used in encoder-decoder architectures with multi-headed self-attention.",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.989,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.7800876589587,
              y: 811.1846398948727,
            },
            {
              x: 1049.4503160462762,
              y: 811.1846398948727,
            },
            {
              x: 1049.4503160462762,
              y: 872.951422555106,
            },
            {
              x: 223.7800876589587,
              y: 872.951422555106,
            },
          ],
          boundingBox: {
            left: 223.7800876589587,
            top: 811.1846398948727,
            right: 1049.4503160462762,
            bottom: 872.951422555106,
          },
        },
        {
          id: "block_10_4PqBx5",
          type: "text",
          content:
            "For translation tasks, the Transformer can be trained significantly faster than architectures based on recurrent or convolutional layers. On both WMT 2014 English-to-German and WMT 2014 English-to-French translation tasks, we achieve a new state of the art. In the former task our best model outperforms even all previously reported ensembles.",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.989,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 223.91187013500797,
              y: 890.9364237390963,
            },
            {
              x: 1049.8108244290318,
              y: 890.9364237390963,
            },
            {
              x: 1049.8108244290318,
              y: 980.2941302536126,
            },
            {
              x: 223.91187013500797,
              y: 980.2941302536126,
            },
          ],
          boundingBox: {
            left: 223.91187013500797,
            top: 890.9364237390963,
            right: 1049.8108244290318,
            bottom: 980.2941302536126,
          },
        },
        {
          id: "block_10_9Aa2OZ",
          type: "text",
          content:
            "We are excited about the future of attention-based models and plan to apply them to other tasks. We plan to extend the Transformer to problems involving input and output modalities other than text and to investigate local, restricted attention mechanisms to efficiently handle large inputs and outputs such as images, audio and video. Making generation less sequential is another research goals of ours.",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.985,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 223.4691397117002,
              y: 993.4156992059005,
            },
            {
              x: 1051.09778633953,
              y: 993.4156992059005,
            },
            {
              x: 1051.09778633953,
              y: 1081.8706514745727,
            },
            {
              x: 223.4691397117002,
              y: 1081.8706514745727,
            },
          ],
          boundingBox: {
            left: 223.4691397117002,
            top: 993.4156992059005,
            right: 1051.09778633953,
            bottom: 1081.8706514745727,
          },
        },
        {
          id: "block_10_eXUU9v",
          type: "text",
          content:
            "The code we used to train and evaluate our models is available at https://github.com/ tensorflow/tensor2tensor.",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.94,
            avgOcrConfidence: 0.99,
          },
          polygon: [
            {
              x: 222.4288912585182,
              y: 1095.2490678371344,
            },
            {
              x: 1050.6089983195284,
              y: 1095.2490678371344,
            },
            {
              x: 1050.6089983195284,
              y: 1136.225321719521,
            },
            {
              x: 222.4288912585182,
              y: 1136.225321719521,
            },
          ],
          boundingBox: {
            left: 222.4288912585182,
            top: 1095.2490678371344,
            right: 1050.6089983195284,
            bottom: 1136.225321719521,
          },
        },
        {
          id: "block_10_HbnFiy",
          type: "text",
          content:
            "Acknowledgements We are grateful to Nal Kalchbrenner and Stephan Gouws for their fruitful comments, corrections and inspiration.",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Text",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.996,
          },
          polygon: [
            {
              x: 223.74068072242457,
              y: 1165.530171673997,
            },
            {
              x: 1049.9162882783987,
              y: 1165.530171673997,
            },
            {
              x: 1049.9162882783987,
              y: 1209.6124608319503,
            },
            {
              x: 223.74068072242457,
              y: 1209.6124608319503,
            },
          ],
          boundingBox: {
            left: 223.74068072242457,
            top: 1165.530171673997,
            right: 1049.9162882783987,
            bottom: 1209.6124608319503,
          },
        },
        {
          id: "block_10_FqHFgA",
          type: "section_heading",
          content: "### References",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Subheading",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 224.3401158465086,
              y: 1243.4205694557131,
            },
            {
              x: 340.41243086766156,
              y: 1243.4205694557131,
            },
            {
              x: 340.41243086766156,
              y: 1263.1433630921786,
            },
            {
              x: 224.3401158465086,
              y: 1263.1433630921786,
            },
          ],
          boundingBox: {
            left: 224.3401158465086,
            top: 1243.4205694557131,
            right: 340.41243086766156,
            bottom: 1263.1433630921786,
          },
        },
        {
          id: "block_10_PnBCYC",
          type: "text",
          content:
            "[1] Jimmy Lei Ba, Jamie Ryan Kiros, and Geoffrey E Hinton. Layer normalization. arXiv preprint arXiv: 1607.06450, 2016.",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.913,
            avgOcrConfidence: 0.988,
          },
          polygon: [
            {
              x: 234.62492392881074,
              y: 1281.5349446346884,
            },
            {
              x: 1049.520940850251,
              y: 1281.5349446346884,
            },
            {
              x: 1049.520940850251,
              y: 1323.6168167573169,
            },
            {
              x: 234.62492392881074,
              y: 1323.6168167573169,
            },
          ],
          boundingBox: {
            left: 234.62492392881074,
            top: 1281.5349446346884,
            right: 1049.520940850251,
            bottom: 1323.6168167573169,
          },
        },
        {
          id: "block_10_sUAYRJ",
          type: "text",
          content:
            "[2] Dzmitry Bahdanau, Kyunghyun Cho, and Yoshua Bengio. Neural machine translation by jointly learning to align and translate. CoRR, abs/1409.0473, 2014.",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.913,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 233.7351792050104,
              y: 1342.1247941210754,
            },
            {
              x: 1048.5688313950586,
              y: 1342.1247941210754,
            },
            {
              x: 1048.5688313950586,
              y: 1385.6842018070079,
            },
            {
              x: 233.7351792050104,
              y: 1385.6842018070079,
            },
          ],
          boundingBox: {
            left: 233.7351792050104,
            top: 1342.1247941210754,
            right: 1048.5688313950586,
            bottom: 1385.6842018070079,
          },
        },
        {
          id: "block_10_cYNyCb",
          type: "text",
          content:
            "[3] Denny Britz, Anna Goldie, Minh-Thang Luong, and Quoc V. Le. Massive exploration of neural machine translation architectures. CoRR, abs/1703.03906, 2017.",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.914,
            avgOcrConfidence: 0.989,
          },
          polygon: [
            {
              x: 234.47555625525703,
              y: 1401.96474766552,
            },
            {
              x: 1050.0915248898693,
              y: 1401.96474766552,
            },
            {
              x: 1050.0915248898693,
              y: 1443.4021335687853,
            },
            {
              x: 234.47555625525703,
              y: 1443.4021335687853,
            },
          ],
          boundingBox: {
            left: 234.47555625525703,
            top: 1401.96474766552,
            right: 1050.0915248898693,
            bottom: 1443.4021335687853,
          },
        },
        {
          id: "block_10_akeHB3",
          type: "text",
          content:
            "[4] Jianpeng Cheng, Li Dong, and Mirella Lapata. Long short-term memory-networks for machine reading. arXiv preprint arXiv: 1601.06733, 2016.",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.91,
            avgOcrConfidence: 0.989,
          },
          polygon: [
            {
              x: 235.1935672063897,
              y: 1461.8473196961825,
            },
            {
              x: 1049.3358584216041,
              y: 1461.8473196961825,
            },
            {
              x: 1049.3358584216041,
              y: 1505.7106445427228,
            },
            {
              x: 235.1935672063897,
              y: 1505.7106445427228,
            },
          ],
          boundingBox: {
            left: 235.1935672063897,
            top: 1461.8473196961825,
            right: 1049.3358584216041,
            bottom: 1505.7106445427228,
          },
        },
        {
          id: "block_10_sJYSgt",
          type: "page_number",
          content: "10",
          metadata: {
            page: {
              number: 10,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.999,
            avgOcrConfidence: 0.999,
          },
          polygon: [
            {
              x: 628.4902252420022,
              y: 1547.7831405983832,
            },
            {
              x: 647.7796484954166,
              y: 1547.7831405983832,
            },
            {
              x: 647.7796484954166,
              y: 1563.3856736950409,
            },
            {
              x: 628.4902252420022,
              y: 1563.3856736950409,
            },
          ],
          boundingBox: {
            left: 628.4902252420022,
            top: 1547.7831405983832,
            right: 647.7796484954166,
            bottom: 1563.3856736950409,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_11_VBbBwR",
          type: "text",
          content:
            "[5] Kyunghyun Cho, Bart van Merrienboer, Caglar Gulcehre, Fethi Bougares, Holger Schwenk, and Yoshua Bengio. Learning phrase representations using rnn encoder-decoder for statistical machine translation. CoRR, abs/1406.1078, 2014.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.932,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 234.95565818174043,
              y: 155.04768656967278,
            },
            {
              x: 1051.617910566121,
              y: 155.04768656967278,
            },
            {
              x: 1051.617910566121,
              y: 219.22101676195186,
            },
            {
              x: 234.95565818174043,
              y: 219.22101676195186,
            },
          ],
          boundingBox: {
            left: 234.95565818174043,
            top: 155.04768656967278,
            right: 1051.617910566121,
            bottom: 219.22101676195186,
          },
        },
        {
          id: "block_11_fZULb0",
          type: "text",
          content:
            "[6] Francois Chollet. Xception: Deep learning with depthwise separable convolutions. arXiv preprint arXiv: 1610.02357, 2016.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.916,
            avgOcrConfidence: 0.987,
          },
          polygon: [
            {
              x: 234.37082610861228,
              y: 242.1581676597882,
            },
            {
              x: 1050.1136779785156,
              y: 242.1581676597882,
            },
            {
              x: 1050.1136779785156,
              y: 284.96119192309845,
            },
            {
              x: 234.37082610861228,
              y: 284.96119192309845,
            },
          ],
          boundingBox: {
            left: 234.37082610861228,
            top: 242.1581676597882,
            right: 1050.1136779785156,
            bottom: 284.96119192309845,
          },
        },
        {
          id: "block_11_KQxaZG",
          type: "text",
          content:
            "[7] Junyoung Chung, \u00c7aglar G\u00fcl\u00e7ehre, Kyunghyun Cho, and Yoshua Bengio. Empirical evaluation of gated recurrent neural networks on sequence modeling. CoRR, abs/1412.3555, 2014.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.937,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 234.59709056102446,
              y: 305.8525607747243,
            },
            {
              x: 1050.252182118214,
              y: 305.8525607747243,
            },
            {
              x: 1050.252182118214,
              y: 348.70033444856347,
            },
            {
              x: 234.59709056102446,
              y: 348.70033444856347,
            },
          ],
          boundingBox: {
            left: 234.59709056102446,
            top: 305.8525607747243,
            right: 1050.252182118214,
            bottom: 348.70033444856347,
          },
        },
        {
          id: "block_11_lSxX1m",
          type: "text",
          content:
            "[8] Chris Dyer, Adhiguna Kuncoro, Miguel Ballesteros, and Noah A. Smith. Recurrent neural network grammars. In Proc. of NAACL, 2016.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.92,
            avgOcrConfidence: 0.99,
          },
          polygon: [
            {
              x: 234.54393261540545,
              y: 369.88837531814,
            },
            {
              x: 1049.7648141679972,
              y: 369.88837531814,
            },
            {
              x: 1049.7648141679972,
              y: 412.63161531606113,
            },
            {
              x: 234.54393261540545,
              y: 412.63161531606113,
            },
          ],
          boundingBox: {
            left: 234.54393261540545,
            top: 369.88837531814,
            right: 1049.7648141679972,
            bottom: 412.63161531606113,
          },
        },
        {
          id: "block_11_3YLXoz",
          type: "text",
          content:
            "[9] Jonas Gehring, Michael Auli, David Grangier, Denis Yarats, and Yann N. Dauphin. Convolu- tional sequence to sequence learning. arXiv preprint arXiv: 1705.03122v2, 2017.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.927,
            avgOcrConfidence: 0.989,
          },
          polygon: [
            {
              x: 233.81299902922916,
              y: 434.3302966813395,
            },
            {
              x: 1052.8067929901345,
              y: 434.3302966813395,
            },
            {
              x: 1052.8067929901345,
              y: 476.945989020785,
            },
            {
              x: 233.81299902922916,
              y: 476.945989020785,
            },
          ],
          boundingBox: {
            left: 233.81299902922916,
            top: 434.3302966813395,
            right: 1052.8067929901345,
            bottom: 476.945989020785,
          },
        },
        {
          id: "block_11_OkxRaO",
          type: "text",
          content:
            "[10] Alex Graves. Generating sequences with recurrent neural networks. arXiv preprint arXiv: 1308.0850, 2013.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.904,
            avgOcrConfidence: 0.986,
          },
          polygon: [
            {
              x: 224.38643378932983,
              y: 498.1091927836712,
            },
            {
              x: 1050.5877919440723,
              y: 498.1091927836712,
            },
            {
              x: 1050.5877919440723,
              y: 539.0395607625632,
            },
            {
              x: 224.38643378932983,
              y: 539.0395607625632,
            },
          ],
          boundingBox: {
            left: 224.38643378932983,
            top: 498.1091927836712,
            right: 1050.5877919440723,
            bottom: 539.0395607625632,
          },
        },
        {
          id: "block_11_RVnHcs",
          type: "text",
          content:
            "[11] Kaiming He, Xiangyu Zhang, Shaoqing Ren, and Jian Sun. Deep residual learning for im- age recognition. In Proceedings of the IEEE Conference on Computer Vision and Pattern Recognition, pages 770-778, 2016.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.919,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 224.3475475450502,
              y: 561.9494121522832,
            },
            {
              x: 1053.1976908662893,
              y: 561.9494121522832,
            },
            {
              x: 1053.1976908662893,
              y: 627.2271530610278,
            },
            {
              x: 224.3475475450502,
              y: 627.2271530610278,
            },
          ],
          boundingBox: {
            left: 224.3475475450502,
            top: 561.9494121522832,
            right: 1053.1976908662893,
            bottom: 627.2271530610278,
          },
        },
        {
          id: "block_11_s96TQd",
          type: "text",
          content:
            "[12] Sepp Hochreiter, Yoshua Bengio, Paolo Frasconi, and J\u00fcrgen Schmidhuber. Gradient flow in recurrent nets: the difficulty of learning long-term dependencies, 2001.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.892,
            avgOcrConfidence: 0.989,
          },
          polygon: [
            {
              x: 224.27874516396628,
              y: 647.9830188034173,
            },
            {
              x: 1048.8635432111087,
              y: 647.9830188034173,
            },
            {
              x: 1048.8635432111087,
              y: 692.1669293740639,
            },
            {
              x: 224.27874516396628,
              y: 692.1669293740639,
            },
          ],
          boundingBox: {
            left: 224.27874516396628,
            top: 647.9830188034173,
            right: 1048.8635432111087,
            bottom: 692.1669293740639,
          },
        },
        {
          id: "block_11_bKrlBK",
          type: "text",
          content:
            "[13] Sepp Hochreiter and J\u00fcrgen Schmidhuber. Long short-term memory. Neural computation, 9(8):1735-1780, 1997.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.903,
            avgOcrConfidence: 0.984,
          },
          polygon: [
            {
              x: 224.36335765532334,
              y: 712.3709330738038,
            },
            {
              x: 1051.7104044447851,
              y: 712.3709330738038,
            },
            {
              x: 1051.7104044447851,
              y: 754.2910917181719,
            },
            {
              x: 224.36335765532334,
              y: 754.2910917181719,
            },
          ],
          boundingBox: {
            left: 224.36335765532334,
            top: 712.3709330738038,
            right: 1051.7104044447851,
            bottom: 754.2910917181719,
          },
        },
        {
          id: "block_11_c3sQ7S",
          type: "text",
          content:
            "[14] Zhongqiang Huang and Mary Harper. Self-training PCFG grammars with latent annotations across languages. In Proceedings of the 2009 Conference on Empirical Methods in Natural Language Processing, pages 832-841. ACL, August 2009.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.908,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 224.70277800177138,
              y: 777.0990849258308,
            },
            {
              x: 1050.1772024335653,
              y: 777.0990849258308,
            },
            {
              x: 1050.1772024335653,
              y: 842.7015786995565,
            },
            {
              x: 224.70277800177138,
              y: 842.7015786995565,
            },
          ],
          boundingBox: {
            left: 224.70277800177138,
            top: 777.0990849258308,
            right: 1050.1772024335653,
            bottom: 842.7015786995565,
          },
        },
        {
          id: "block_11_HmSxMA",
          type: "text",
          content:
            "[15] Rafal Jozefowicz, Oriol Vinyals, Mike Schuster, Noam Shazeer, and Yonghui Wu. Exploring the limits of language modeling. arXiv preprint arXiv: 1602.02410, 2016.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.906,
            avgOcrConfidence: 0.989,
          },
          polygon: [
            {
              x: 223.96287430811972,
              y: 863.015632801486,
            },
            {
              x: 1049.822942357864,
              y: 863.015632801486,
            },
            {
              x: 1049.822942357864,
              y: 907.0369301702743,
            },
            {
              x: 223.96287430811972,
              y: 907.0369301702743,
            },
          ],
          boundingBox: {
            left: 223.96287430811972,
            top: 863.015632801486,
            right: 1049.822942357864,
            bottom: 907.0369301702743,
          },
        },
        {
          id: "block_11_XlCObz",
          type: "text",
          content:
            "[16] \u0141ukasz Kaiser and Samy Bengio. Can active memory replace attention? In Advances in Neural Information Processing Systems, (NIPS), 2016.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.902,
            avgOcrConfidence: 0.988,
          },
          polygon: [
            {
              x: 224.17545875493627,
              y: 927.1137413655905,
            },
            {
              x: 1050.2528448174469,
              y: 927.1137413655905,
            },
            {
              x: 1050.2528448174469,
              y: 970.5487977795134,
            },
            {
              x: 224.17545875493627,
              y: 970.5487977795134,
            },
          ],
          boundingBox: {
            left: 224.17545875493627,
            top: 927.1137413655905,
            right: 1050.2528448174469,
            bottom: 970.5487977795134,
          },
        },
        {
          id: "block_11_48mMcn",
          type: "text",
          content:
            "[17] \u0141ukasz Kaiser and Ilya Sutskever. Neural GPUs learn algorithms. In International Conference on Learning Representations (ICLR), 2016.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.903,
            avgOcrConfidence: 0.987,
          },
          polygon: [
            {
              x: 224.42198286961465,
              y: 991.1752927392946,
            },
            {
              x: 1049.7810976348653,
              y: 991.1752927392946,
            },
            {
              x: 1049.7810976348653,
              y: 1034.8329123590227,
            },
            {
              x: 224.42198286961465,
              y: 1034.8329123590227,
            },
          ],
          boundingBox: {
            left: 224.42198286961465,
            top: 991.1752927392946,
            right: 1049.7810976348653,
            bottom: 1034.8329123590227,
          },
        },
        {
          id: "block_11_lBImir",
          type: "text",
          content:
            "[18] Nal Kalchbrenner, Lasse Espeholt, Karen Simonyan, Aaron van den Oord, Alex Graves, and Ko- ray Kavukcuoglu. Neural machine translation in linear time. arXiv preprint arXiv: 1610.10099v2, 2017.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.934,
            avgOcrConfidence: 0.988,
          },
          polygon: [
            {
              x: 224.23202486803933,
              y: 1055.1385374714557,
            },
            {
              x: 1052.4190192675069,
              y: 1055.1385374714557,
            },
            {
              x: 1052.4190192675069,
              y: 1117.975328259002,
            },
            {
              x: 224.23202486803933,
              y: 1117.975328259002,
            },
          ],
          boundingBox: {
            left: 224.23202486803933,
            top: 1055.1385374714557,
            right: 1052.4190192675069,
            bottom: 1117.975328259002,
          },
        },
        {
          id: "block_11_Y0ddKF",
          type: "text",
          content:
            "[19] Yoon Kim, Carl Denton, Luong Hoang, and Alexander M. Rush. Structured attention networks. In International Conference on Learning Representations, 2017.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.974,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 224.29036606837366,
              y: 1142.1438929622334,
            },
            {
              x: 1052.7023705252766,
              y: 1142.1438929622334,
            },
            {
              x: 1052.7023705252766,
              y: 1185.793178300212,
            },
            {
              x: 224.29036606837366,
              y: 1185.793178300212,
            },
          ],
          boundingBox: {
            left: 224.29036606837366,
            top: 1142.1438929622334,
            right: 1052.7023705252766,
            bottom: 1185.793178300212,
          },
        },
        {
          id: "block_11_nehypx",
          type: "text",
          content:
            "[20] Diederik Kingma and Jimmy Ba. Adam: A method for stochastic optimization. In ICLR, 2015.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.912,
            avgOcrConfidence: 0.989,
          },
          polygon: [
            {
              x: 223.92950266817192,
              y: 1205.8213097134928,
            },
            {
              x: 1052.7550077786411,
              y: 1205.8213097134928,
            },
            {
              x: 1052.7550077786411,
              y: 1227.2851158658366,
            },
            {
              x: 223.92950266817192,
              y: 1227.2851158658366,
            },
          ],
          boundingBox: {
            left: 223.92950266817192,
            top: 1205.8213097134928,
            right: 1052.7550077786411,
            bottom: 1227.2851158658366,
          },
        },
        {
          id: "block_11_af7MVz",
          type: "text",
          content:
            "[21] Oleksii Kuchaiev and Boris Ginsburg. Factorization tricks for LSTM networks. arXiv preprint arXiv: 1703.10722, 2017.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.925,
            avgOcrConfidence: 0.989,
          },
          polygon: [
            {
              x: 223.78512890669552,
              y: 1247.11833873548,
            },
            {
              x: 1050.4177622551465,
              y: 1247.11833873548,
            },
            {
              x: 1050.4177622551465,
              y: 1289.109575544085,
            },
            {
              x: 223.78512890669552,
              y: 1289.109575544085,
            },
          ],
          boundingBox: {
            left: 223.78512890669552,
            top: 1247.11833873548,
            right: 1050.4177622551465,
            bottom: 1289.109575544085,
          },
        },
        {
          id: "block_11_vnPlfZ",
          type: "text",
          content:
            "[22] Zhouhan Lin, Minwei Feng, Cicero Nogueira dos Santos, Mo Yu, Bing Xiang, Bowen Zhou, and Yoshua Bengio. A structured self-attentive sentence embedding. arXiv preprint arXiv: 1703.03130, 2017.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.913,
            avgOcrConfidence: 0.99,
          },
          polygon: [
            {
              x: 223.74063338676507,
              y: 1311.0246641116034,
            },
            {
              x: 1050.2205618976677,
              y: 1311.0246641116034,
            },
            {
              x: 1050.2205618976677,
              y: 1375.8748468958345,
            },
            {
              x: 223.74063338676507,
              y: 1375.8748468958345,
            },
          ],
          boundingBox: {
            left: 223.74063338676507,
            top: 1311.0246641116034,
            right: 1050.2205618976677,
            bottom: 1375.8748468958345,
          },
        },
        {
          id: "block_11_g0SHxO",
          type: "text",
          content:
            "[23] Minh-Thang Luong, Quoc V. Le, Ilya Sutskever, Oriol Vinyals, and Lukasz Kaiser. Multi-task sequence to sequence learning. arXiv preprint arXiv: 1511.06114, 2015.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.908,
            avgOcrConfidence: 0.988,
          },
          polygon: [
            {
              x: 224.355428932357,
              y: 1397.7982697451025,
            },
            {
              x: 1050.4997476174015,
              y: 1397.7982697451025,
            },
            {
              x: 1050.4997476174015,
              y: 1441.9434448471643,
            },
            {
              x: 224.355428932357,
              y: 1441.9434448471643,
            },
          ],
          boundingBox: {
            left: 224.355428932357,
            top: 1397.7982697451025,
            right: 1050.4997476174015,
            bottom: 1441.9434448471643,
          },
        },
        {
          id: "block_11_gDtqYI",
          type: "text",
          content:
            "[24] Minh-Thang Luong, Hieu Pham, and Christopher D Manning. Effective approaches to attention- based neural machine translation. arXiv preprint arXiv: 1508.04025, 2015.",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.927,
            avgOcrConfidence: 0.99,
          },
          polygon: [
            {
              x: 224.7064228475529,
              y: 1461.612349775501,
            },
            {
              x: 1052.9802308465444,
              y: 1461.612349775501,
            },
            {
              x: 1052.9802308465444,
              y: 1506.2439438669305,
            },
            {
              x: 224.7064228475529,
              y: 1506.2439438669305,
            },
          ],
          boundingBox: {
            left: 224.7064228475529,
            top: 1461.612349775501,
            right: 1052.9802308465444,
            bottom: 1506.2439438669305,
          },
        },
        {
          id: "block_11_JWTkzE",
          type: "page_number",
          content: "11",
          metadata: {
            page: {
              number: 11,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.995,
            avgOcrConfidence: 0.995,
          },
          polygon: [
            {
              x: 629.039650241824,
              y: 1547.854550239735,
            },
            {
              x: 645.38100862155,
              y: 1547.854550239735,
            },
            {
              x: 645.38100862155,
              y: 1563.4467601919532,
            },
            {
              x: 629.039650241824,
              y: 1563.4467601919532,
            },
          ],
          boundingBox: {
            left: 629.039650241824,
            top: 1547.854550239735,
            right: 645.38100862155,
            bottom: 1563.4467601919532,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_12_TQ4Lbe",
          type: "text",
          content:
            "[25] Mitchell P Marcus, Mary Ann Marcinkiewicz, and Beatrice Santorini. Building a large annotated corpus of english: The penn treebank. Computational linguistics, 19(2):313-330, 1993.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.913,
            avgOcrConfidence: 0.99,
          },
          polygon: [
            {
              x: 223.94512343580706,
              y: 154.85743054411464,
            },
            {
              x: 1050.2930801280222,
              y: 154.85743054411464,
            },
            {
              x: 1050.2930801280222,
              y: 198.76930494953817,
            },
            {
              x: 223.94512343580706,
              y: 198.76930494953817,
            },
          ],
          boundingBox: {
            left: 223.94512343580706,
            top: 154.85743054411464,
            right: 1050.2930801280222,
            bottom: 198.76930494953817,
          },
        },
        {
          id: "block_12_LvCkff",
          type: "text",
          content:
            "[26] David McClosky, Eugene Charniak, and Mark Johnson. Effective self-training for parsing. In Proceedings of the Human Language Technology Conference of the NAACL, Main Conference, pages 152-159. ACL, June 2006.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.907,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 224.21323261121762,
              y: 226.782506971431,
            },
            {
              x: 1051.9384676522582,
              y: 226.782506971431,
            },
            {
              x: 1051.9384676522582,
              y: 292.8687442406676,
            },
            {
              x: 224.21323261121762,
              y: 292.8687442406676,
            },
          ],
          boundingBox: {
            left: 224.21323261121762,
            top: 226.782506971431,
            right: 1051.9384676522582,
            bottom: 292.8687442406676,
          },
        },
        {
          id: "block_12_Rm8AFL",
          type: "text",
          content:
            "[27] Ankur Parikh, Oscar T\u00e4ckstr\u00f6m, Dipanjan Das, and Jakob Uszkoreit. A decomposable attention model. In Empirical Methods in Natural Language Processing, 2016.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.916,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 224.08847948060418,
              y: 321.2656073462694,
            },
            {
              x: 1048.8436622341185,
              y: 321.2656073462694,
            },
            {
              x: 1048.8436622341185,
              y: 364.85913349811295,
            },
            {
              x: 224.08847948060418,
              y: 364.85913349811295,
            },
          ],
          boundingBox: {
            left: 224.08847948060418,
            top: 321.2656073462694,
            right: 1048.8436622341185,
            bottom: 364.85913349811295,
          },
        },
        {
          id: "block_12_cqas9e",
          type: "text",
          content:
            "[28] Romain Paulus, Caiming Xiong, and Richard Socher. A deep reinforced model for abstractive summarization. arXiv preprint arXiv: 1705.04304, 2017.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.912,
            avgOcrConfidence: 0.988,
          },
          polygon: [
            {
              x: 224.67392691730583,
              y: 392.7383714116606,
            },
            {
              x: 1049.3349117084142,
              y: 392.7383714116606,
            },
            {
              x: 1049.3349117084142,
              y: 435.42904860991285,
            },
            {
              x: 224.67392691730583,
              y: 435.42904860991285,
            },
          ],
          boundingBox: {
            left: 224.67392691730583,
            top: 392.7383714116606,
            right: 1049.3349117084142,
            bottom: 435.42904860991285,
          },
        },
        {
          id: "block_12_hfa5dB",
          type: "text",
          content:
            "[29] Slav Petrov, Leon Barrett, Romain Thibaux, and Dan Klein. Learning accurate, compact, and interpretable tree annotation. In Proceedings of the 21st International Conference on Computational Linguistics and 44th Annual Meeting of the ACL, pages 433-440. ACL, July 2006.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.899,
            avgOcrConfidence: 0.99,
          },
          polygon: [
            {
              x: 223.91392923619625,
              y: 463.67279296530813,
            },
            {
              x: 1051.5214404920591,
              y: 463.67279296530813,
            },
            {
              x: 1051.5214404920591,
              y: 549.7230681799408,
            },
            {
              x: 223.91392923619625,
              y: 549.7230681799408,
            },
          ],
          boundingBox: {
            left: 223.91392923619625,
            top: 463.67279296530813,
            right: 1051.5214404920591,
            bottom: 549.7230681799408,
          },
        },
        {
          id: "block_12_H2k7w4",
          type: "text",
          content:
            "[30] Ofir Press and Lior Wolf. Using the output embedding to improve language models. arXiv preprint arXiv: 1608.05859, 2016.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.898,
            avgOcrConfidence: 0.987,
          },
          polygon: [
            {
              x: 224.29183347381814,
              y: 581.8894180068396,
            },
            {
              x: 1049.7876299558764,
              y: 581.8894180068396,
            },
            {
              x: 1049.7876299558764,
              y: 624.918793785841,
            },
            {
              x: 224.29183347381814,
              y: 624.918793785841,
            },
          ],
          boundingBox: {
            left: 224.29183347381814,
            top: 581.8894180068396,
            right: 1049.7876299558764,
            bottom: 624.918793785841,
          },
        },
        {
          id: "block_12_UrZIEK",
          type: "text",
          content:
            "[31] Rico Sennrich, Barry Haddow, and Alexandra Birch. Neural machine translation of rare words with subword units. arXiv preprint arXiv: 1508.07909, 2015.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.925,
            avgOcrConfidence: 0.988,
          },
          polygon: [
            {
              x: 224.35140540129947,
              y: 653.3260510666926,
            },
            {
              x: 1049.4855337769445,
              y: 653.3260510666926,
            },
            {
              x: 1049.4855337769445,
              y: 696.2955715494944,
            },
            {
              x: 224.35140540129947,
              y: 696.2955715494944,
            },
          ],
          boundingBox: {
            left: 224.35140540129947,
            top: 653.3260510666926,
            right: 1049.4855337769445,
            bottom: 696.2955715494944,
          },
        },
        {
          id: "block_12_IIwlxZ",
          type: "text",
          content:
            "[32] Noam Shazeer, Azalia Mirhoseini, Krzysztof Maziarz, Andy Davis, Quoc Le, Geoffrey Hinton, and Jeff Dean. Outrageously large neural networks: The sparsely-gated mixture-of-experts layer. arXiv preprint arXiv: 1701.06538, 2017.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.88,
            avgOcrConfidence: 0.988,
          },
          polygon: [
            {
              x: 224.15238262092979,
              y: 724.7319041087216,
            },
            {
              x: 1051.274348349467,
              y: 724.7319041087216,
            },
            {
              x: 1051.274348349467,
              y: 791.0497254966793,
            },
            {
              x: 224.15238262092979,
              y: 791.0497254966793,
            },
          ],
          boundingBox: {
            left: 224.15238262092979,
            top: 724.7319041087216,
            right: 1051.274348349467,
            bottom: 791.0497254966793,
          },
        },
        {
          id: "block_12_pGLzhM",
          type: "text",
          content:
            "[33] Nitish Srivastava, Geoffrey E Hinton, Alex Krizhevsky, Ilya Sutskever, and Ruslan Salakhutdi- nov. Dropout: a simple way to prevent neural networks from overfitting. Journal of Machine Learning Research, 15(1):1929-1958, 2014.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.938,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 224.4417928431156,
              y: 819.0001126052741,
            },
            {
              x: 1053.3042907714844,
              y: 819.0001126052741,
            },
            {
              x: 1053.3042907714844,
              y: 885.2761205228649,
            },
            {
              x: 224.4417928431156,
              y: 885.2761205228649,
            },
          ],
          boundingBox: {
            left: 224.4417928431156,
            top: 819.0001126052741,
            right: 1053.3042907714844,
            bottom: 885.2761205228649,
          },
        },
        {
          id: "block_12_Bz8MfK",
          type: "text",
          content:
            "[34] Sainbayar Sukhbaatar, Arthur Szlam, Jason Weston, and Rob Fergus. End-to-end memory networks. In C. Cortes, N. D. Lawrence, D. D. Lee, M. Sugiyama, and R. Garnett, editors, Advances in Neural Information Processing Systems 28, pages 2440-2448. Curran Associates, Inc., 2015.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.897,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 224.0948934624665,
              y: 913.8373252467105,
            },
            {
              x: 1051.823347328353,
              y: 913.8373252467105,
            },
            {
              x: 1051.823347328353,
              y: 1000.8878563328793,
            },
            {
              x: 224.0948934624665,
              y: 1000.8878563328793,
            },
          ],
          boundingBox: {
            left: 224.0948934624665,
            top: 913.8373252467105,
            right: 1051.823347328353,
            bottom: 1000.8878563328793,
          },
        },
        {
          id: "block_12_m0nnSr",
          type: "text",
          content:
            "[35] Ilya Sutskever, Oriol Vinyals, and Quoc VV Le. Sequence to sequence learning with neural networks. In Advances in Neural Information Processing Systems, pages 3104-3112, 2014.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.899,
            avgOcrConfidence: 0.988,
          },
          polygon: [
            {
              x: 224.52872478178818,
              y: 1030.965426796361,
            },
            {
              x: 1050.1110271815835,
              y: 1030.965426796361,
            },
            {
              x: 1050.1110271815835,
              y: 1074.9609636579241,
            },
            {
              x: 224.52872478178818,
              y: 1074.9609636579241,
            },
          ],
          boundingBox: {
            left: 224.52872478178818,
            top: 1030.965426796361,
            right: 1050.1110271815835,
            bottom: 1074.9609636579241,
          },
        },
        {
          id: "block_12_C1nzdD",
          type: "text",
          content:
            "[36] Christian Szegedy, Vincent Vanhoucke, Sergey Ioffe, Jonathon Shlens, and Zbigniew Wojna. Rethinking the inception architecture for computer vision. CoRR, abs/1512.00567, 2015.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.939,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 224.60145602261062,
              y: 1102.9526906981505,
            },
            {
              x: 1051.9790816481095,
              y: 1102.9526906981505,
            },
            {
              x: 1051.9790816481095,
              y: 1146.15116715969,
            },
            {
              x: 224.60145602261062,
              y: 1146.15116715969,
            },
          ],
          boundingBox: {
            left: 224.60145602261062,
            top: 1102.9526906981505,
            right: 1051.9790816481095,
            bottom: 1146.15116715969,
          },
        },
        {
          id: "block_12_smJ629",
          type: "text",
          content:
            "[37] Vinyals & Kaiser, Koo, Petrov, Sutskever, and Hinton. Grammar as a foreign language. In Advances in Neural Information Processing Systems, 2015.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.912,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 224.70363004364236,
              y: 1173.716330506748,
            },
            {
              x: 1049.93891472364,
              y: 1173.716330506748,
            },
            {
              x: 1049.93891472364,
              y: 1218.0808487512115,
            },
            {
              x: 224.70363004364236,
              y: 1218.0808487512115,
            },
          ],
          boundingBox: {
            left: 224.70363004364236,
            top: 1173.716330506748,
            right: 1049.93891472364,
            bottom: 1218.0808487512115,
          },
        },
        {
          id: "block_12_oKrqzS",
          type: "text",
          content:
            "[38] Yonghui Wu, Mike Schuster, Zhifeng Chen, Quoc V Le, Mohammad Norouzi, Wolfgang Macherey, Maxim Krikun, Yuan Cao, Qin Gao, Klaus Macherey, et al. Google's neural machine translation system: Bridging the gap between human and machine translation. arXiv preprint arXiv: 1609.08144, 2016.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.935,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 224.222865417926,
              y: 1246.0137622804568,
            },
            {
              x: 1050.4285547855127,
              y: 1246.0137622804568,
            },
            {
              x: 1050.4285547855127,
              y: 1332.3901636224045,
            },
            {
              x: 224.222865417926,
              y: 1332.3901636224045,
            },
          ],
          boundingBox: {
            left: 224.222865417926,
            top: 1246.0137622804568,
            right: 1050.4285547855127,
            bottom: 1332.3901636224045,
          },
        },
        {
          id: "block_12_5hQUgX",
          type: "text",
          content:
            "[39] Jie Zhou, Ying Cao, Xuguang Wang, Peng Li, and Wei Xu. Deep recurrent models with fast-forward connections for neural machine translation. CoRR, abs/1606.04199, 2016.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.977,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 224.40534438530025,
              y: 1362.7727877394598,
            },
            {
              x: 1049.6643678985372,
              y: 1362.7727877394598,
            },
            {
              x: 1049.6643678985372,
              y: 1404.796982844073,
            },
            {
              x: 224.40534438530025,
              y: 1404.796982844073,
            },
          ],
          boundingBox: {
            left: 224.40534438530025,
            top: 1362.7727877394598,
            right: 1049.6643678985372,
            bottom: 1404.796982844073,
          },
        },
        {
          id: "block_12_7qyMnf",
          type: "text",
          content:
            "[40] Muhua Zhu, Yue Zhang, Wenliang Chen, Min Zhang, and Jingbo Zhu. Fast and accurate shift-reduce constituent parsing. In Proceedings of the 51st Annual Meeting of the ACL (Volume 1: Long Papers), pages 434-443. ACL, August 2013.",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "List Item",
            minOcrConfidence: 0.905,
            avgOcrConfidence: 0.991,
          },
          polygon: [
            {
              x: 224.04902520841057,
              y: 1434.5126750056904,
            },
            {
              x: 1049.9945814592124,
              y: 1434.5126750056904,
            },
            {
              x: 1049.9945814592124,
              y: 1501.148572362455,
            },
            {
              x: 224.04902520841057,
              y: 1501.148572362455,
            },
          ],
          boundingBox: {
            left: 224.04902520841057,
            top: 1434.5126750056904,
            right: 1049.9945814592124,
            bottom: 1501.148572362455,
          },
        },
        {
          id: "block_12_6o6JUd",
          type: "page_number",
          content: "12",
          metadata: {
            page: {
              number: 12,
              width: 1275,
              height: 1651,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.998,
            avgOcrConfidence: 0.998,
          },
          polygon: [
            {
              x: 628.997000812614,
              y: 1547.789391309695,
            },
            {
              x: 647.0427269483133,
              y: 1547.789391309695,
            },
            {
              x: 647.0427269483133,
              y: 1563.398364533159,
            },
            {
              x: 628.997000812614,
              y: 1563.398364533159,
            },
          ],
          boundingBox: {
            left: 628.997000812614,
            top: 1547.789391309695,
            right: 647.0427269483133,
            bottom: 1563.398364533159,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_13_KlKk5h",
          type: "page_number",
          content: "13",
          metadata: {
            page: {
              number: 13,
              width: 1651,
              height: 1275,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.999,
            avgOcrConfidence: 0.999,
          },
          polygon: [
            {
              x: 103.29433033878627,
              y: 628.7723457726248,
            },
            {
              x: 103.29433033878627,
              y: 646.9001519418981,
            },
            {
              x: 87.59273293860883,
              y: 646.9001519418981,
            },
            {
              x: 87.59273293860883,
              y: 628.7723457726248,
            },
          ],
          boundingBox: {
            left: 87.59273293860883,
            top: 628.7723457726248,
            right: 103.29433033878627,
            bottom: 646.9001519418981,
          },
        },
        {
          id: "block_13_PkaOm5",
          type: "text",
          content:
            "the word 'making'. Different colors represent different heads. Best viewed in color. the verb 'making', completing the phrase 'making ... more difficult'. Attentions here shown only for encoder self-attention in layer 5 of 6. Many of the attention heads attend to a distant dependency of Figure 3: An example of the attention mechanism following long-distance dependencies in the",
          metadata: {
            page: {
              number: 13,
              width: 1651,
              height: 1275,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.937,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 999.8870322148603,
              y: 224.33585563715354,
            },
            {
              x: 999.8870322148603,
              y: 1052.0067256732577,
            },
            {
              x: 911.037953656419,
              y: 1052.0067256732577,
            },
            {
              x: 911.037953656419,
              y: 224.33585563715354,
            },
          ],
          boundingBox: {
            left: 911.037953656419,
            top: 224.33585563715354,
            right: 999.8870322148603,
            bottom: 1052.0067256732577,
          },
        },
        {
          id: "block_13_xamkHQ",
          type: "figure",
          content:
            "<figure type=\"diagram\">\nIt It is is in in this this spirit spirit that that a a majority majority of of American American governments governments have have passed passed new new laws laws since since 2009 2009 making making the the registration registration or or voting voting process process more more difficult difficult\n\n. . <EOS> <EOS> <pad> <pad> <pad> <pad> <pad> <pad> <pad> <pad> <pad> <pad> <pad> <pad>\n<caption>Diagram showing attention mechanism or word alignment between two identical sequences of text. The word 'making' in the right column is highlighted in gray and acts as the target for connections from the left column. Connections originate from the words 'laws', '2009', 'making', 'more', and 'difficult' in the left column and point to 'making' in the right column. The sequence of text is: 'It is in this spirit that a majority of American governments have passed new laws since 2009 making the registration or voting process more difficult. <EOS> <pad> <pad> <pad> <pad> <pad> <pad> <pad>'</caption>\n</figure>",
          metadata: {
            page: {
              number: 13,
              width: 1651,
              height: 1275,
            },
            layoutClass: "Picture/Figure/Image/Chart",
            minOcrConfidence: 0.717,
            avgOcrConfidence: 0.98,
          },
          polygon: [
            {
              x: 1444.0009233001479,
              y: 252.8656019781628,
            },
            {
              x: 1444.0009233001479,
              y: 1047.5539548553688,
            },
            {
              x: 1022.9538143416097,
              y: 1047.5539548553688,
            },
            {
              x: 1022.9538143416097,
              y: 252.8656019781628,
            },
          ],
          boundingBox: {
            left: 1022.9538143416097,
            top: 252.8656019781628,
            right: 1444.0009233001479,
            bottom: 1047.5539548553688,
          },
        },
        {
          id: "block_13_5eYqPZ",
          type: "heading",
          content: "# Attention Visualizations",
          metadata: {
            page: {
              number: 13,
              width: 1651,
              height: 1275,
            },
            layoutClass: "Heading",
            minOcrConfidence: 0.992,
            avgOcrConfidence: 0.992,
          },
          polygon: [
            {
              x: 1498.6412794643775,
              y: 224.94428453654268,
            },
            {
              x: 1498.6412794643775,
              y: 480.8453803514912,
            },
            {
              x: 1479.199057528847,
              y: 480.8453803514912,
            },
            {
              x: 1479.199057528847,
              y: 224.94428453654268,
            },
          ],
          boundingBox: {
            left: 1479.199057528847,
            top: 224.94428453654268,
            right: 1498.6412794643775,
            bottom: 480.8453803514912,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_14_DaAuYi",
          type: "page_number",
          content: "14",
          metadata: {
            page: {
              number: 14,
              width: 1651,
              height: 1275,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.998,
            avgOcrConfidence: 0.998,
          },
          polygon: [
            {
              x: 103.12243577770732,
              y: 628.7327731612825,
            },
            {
              x: 103.12243577770732,
              y: 647.0026336447166,
            },
            {
              x: 87.74237117910752,
              y: 647.0026336447166,
            },
            {
              x: 87.74237117910752,
              y: 628.7327731612825,
            },
          ],
          boundingBox: {
            left: 87.74237117910752,
            top: 628.7327731612825,
            right: 103.12243577770732,
            bottom: 647.0026336447166,
          },
        },
        {
          id: "block_14_VDKMys",
          type: "text",
          content:
            "and 6. Note that the attentions are very sharp for this word. Full attentions for head 5. Bottom: Isolated attentions from just the word 'its' for attention heads 5 Figure 4: Two attention heads, also in layer 5 of 6, apparently involved in anaphora resolution. Top:",
          metadata: {
            page: {
              number: 14,
              width: 1651,
              height: 1275,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.972,
            avgOcrConfidence: 0.993,
          },
          polygon: [
            {
              x: 370.4634644357782,
              y: 224.1528559775248,
            },
            {
              x: 370.4634644357782,
              y: 1051.3102287793681,
            },
            {
              x: 303.81733864232115,
              y: 1051.3102287793681,
            },
            {
              x: 303.81733864232115,
              y: 224.1528559775248,
            },
          ],
          boundingBox: {
            left: 303.81733864232115,
            top: 224.1528559775248,
            right: 370.4634644357782,
            bottom: 1051.3102287793681,
          },
        },
        {
          id: "block_14_QRz2LE",
          type: "figure",
          content:
            "<figure type=\"diagram\">\nThe The The The Law Law Law Law will will will will never never never never be be be be perfect perfect perfect perfect , , , , but but but but its its its its application application application application should should should should be be be be just just just just - - - - this this this this is is is is what what what what we we we we are are are are missing missing missing missing , , , , in in in in my my my my opinion opinion opinion opinion . . . . <EOS> <EOS> <EOS> <EOS> <pad> <pad> <pad> <pad>\n<caption>Attention weight visualization diagram showing alignment between two sequences of text tokens. The diagram consists of three columns of text, with lines connecting tokens between the columns to represent attention strength. Thicker, more opaque lines indicate higher attention weights. The text sequence is: 'The', 'Law', 'will', 'never', 'be', 'perfect', ',', 'but', 'its', 'application', 'should', 'be', 'just', '-', 'this', 'is', 'what', 'we', 'are', 'missing', ',', 'in', 'my', 'opinion', '.', '<EOS>', '<pad>'. Specific highlights are present: the word 'Law' in the first column and 'its' in the second column are highlighted with a purple/grey background, and lines connect these highlighted tokens to corresponding tokens in the third column.</caption>\n</figure>",
          metadata: {
            page: {
              number: 14,
              width: 1651,
              height: 1275,
            },
            layoutClass: "Picture/Figure/Image/Chart",
            minOcrConfidence: 0.379,
            avgOcrConfidence: 0.963,
          },
          polygon: [
            {
              x: 1294.7974623916741,
              y: 255.12043611846704,
            },
            {
              x: 1294.7974623916741,
              y: 1044.444475382784,
            },
            {
              x: 388.9633916410289,
              y: 1044.444475382784,
            },
            {
              x: 388.9633916410289,
              y: 255.12043611846704,
            },
          ],
          boundingBox: {
            left: 388.9633916410289,
            top: 255.12043611846704,
            right: 1294.7974623916741,
            bottom: 1044.444475382784,
          },
        },
      ],
    },
    {
      blocks: [
        {
          id: "block_15_pV91a2",
          type: "page_number",
          content: "15",
          metadata: {
            page: {
              number: 15,
              width: 1651,
              height: 1275,
            },
            layoutClass: "Page Number",
            minOcrConfidence: 0.998,
            avgOcrConfidence: 0.998,
          },
          polygon: [
            {
              x: 103.26014084206486,
              y: 628.7008215911197,
            },
            {
              x: 103.26014084206486,
              y: 647.1549124613296,
            },
            {
              x: 87.58487219559515,
              y: 647.1549124613296,
            },
            {
              x: 87.58487219559515,
              y: 628.7008215911197,
            },
          ],
          boundingBox: {
            left: 87.58487219559515,
            top: 628.7008215911197,
            right: 103.26014084206486,
            bottom: 647.1549124613296,
          },
        },
        {
          id: "block_15_2igWoi",
          type: "text",
          content:
            "at layer 5 of 6. The heads clearly learned to perform different tasks. sentence. We give two such examples above, from two different heads from the encoder self-attention Figure 5: Many of the attention heads exhibit behaviour that seems related to the structure of the",
          metadata: {
            page: {
              number: 15,
              width: 1651,
              height: 1275,
            },
            layoutClass: "Caption",
            minOcrConfidence: 0.976,
            avgOcrConfidence: 0.994,
          },
          polygon: [
            {
              x: 394.6980404387739,
              y: 223.74716570777616,
            },
            {
              x: 394.6980404387739,
              y: 1049.736980800211,
            },
            {
              x: 328.87227315113955,
              y: 1049.736980800211,
            },
            {
              x: 328.87227315113955,
              y: 223.74716570777616,
            },
          ],
          boundingBox: {
            left: 328.87227315113955,
            top: 223.74716570777616,
            right: 394.6980404387739,
            bottom: 1049.736980800211,
          },
        },
        {
          id: "block_15_OqCqJD",
          type: "figure",
          content:
            "<figure type=\"diagram\">\nThe The Law Law will will never never be be perfect perfect , , but but its its application application should should be be just just - - this this is is what what we we are are missing missing , , in in my my opinion opinion\n\n. . <EOS> <EOS> <pad> <pad>\n<caption>Attention weight visualization showing connections between two identical sequences of text: 'The Law will never be perfect , but its application should be just - this is what we are missing , in my opinion . <EOS> <pad>'. The diagram uses lines of varying thickness and red color intensity to indicate the strength of attention between words in the left sequence and words in the right sequence. Stronger connections are represented by thicker, more opaque red lines, while weaker connections are represented by thinner, more transparent red lines.</caption>\n</figure>",
          metadata: {
            page: {
              number: 15,
              width: 1651,
              height: 1275,
            },
            layoutClass: "Picture/Figure/Image/Chart",
            minOcrConfidence: 0.651,
            avgOcrConfidence: 0.969,
          },
          polygon: [
            {
              x: 810.314180990807,
              y: 253.4944325467966,
            },
            {
              x: 810.314180990807,
              y: 1035.3522419059364,
            },
            {
              x: 421.96601087527165,
              y: 1035.3522419059364,
            },
            {
              x: 421.96601087527165,
              y: 253.4944325467966,
            },
          ],
          boundingBox: {
            left: 421.96601087527165,
            top: 253.4944325467966,
            right: 810.314180990807,
            bottom: 1035.3522419059364,
          },
        },
        {
          id: "block_15_sK8Nre",
          type: "figure",
          content:
            "<figure type=\"diagram\">\nThe The Law Law will will never never be be perfect perfect , , but but its its application application should should be be just just - - this this is is what what we we are are missing missing , , in in my my opinion opinion\n\n. . <EOS> <EOS> <pad> <pad>\n<caption>Attention weight visualization diagram showing connections between two identical sequences of text: 'The', 'Law', 'will', 'never', 'be', 'perfect', ',', 'but', 'its', 'application', 'should', 'be', 'just', '-', 'this', 'is', 'what', 'we', 'are', 'missing', ',', 'in', 'my', 'opinion', '.', '<EOS>', '<pad>'. The diagram displays lines connecting tokens from the left sequence to the right sequence, where line thickness and opacity represent the strength of the attention weight. Notable high-attention connections include: 'Law' to 'Law', 'application' to 'application', 'missing' to 'missing', 'opinion' to 'opinion', and '<pad>' to '<pad>'.</caption>\n</figure>",
          metadata: {
            page: {
              number: 15,
              width: 1651,
              height: 1275,
            },
            layoutClass: "Picture/Figure/Image/Chart",
            minOcrConfidence: 0.6,
            avgOcrConfidence: 0.968,
          },
          polygon: [
            {
              x: 1255.9501913113702,
              y: 254.50874105857238,
            },
            {
              x: 1255.9501913113702,
              y: 1034.3072598868043,
            },
            {
              x: 882.2701913360366,
              y: 1034.3072598868043,
            },
            {
              x: 882.2701913360366,
              y: 254.50874105857238,
            },
          ],
          boundingBox: {
            left: 882.2701913360366,
            top: 254.50874105857238,
            right: 1255.9501913113702,
            bottom: 1034.3072598868043,
          },
        },
      ],
    },
  ],
} satisfies ParsedOcrOutput

const BLOCK_STYLES: Record<
  OcrBlockType,
  {
    label: string
    icon: React.ComponentProps<typeof HugeiconsIcon>["icon"]
    overlay: string
    mutedOverlay: string
    ring: string
    badge: string
  }
> = {
  heading: {
    label: "Heading",
    icon: Heading01Icon,
    overlay: "border-violet-500/70 bg-violet-500/10",
    mutedOverlay: "border-violet-500/35 bg-violet-500/5",
    ring: "border-violet-500/60 bg-violet-500/5 text-violet-600",
    badge:
      "bg-violet-50 text-violet-600 dark:bg-violet-300/10 dark:text-violet-300",
  },
  paragraph: {
    label: "Paragraph",
    icon: ParagraphIcon,
    overlay: "border-blue-500/70 bg-blue-500/10",
    mutedOverlay: "border-blue-500/35 bg-blue-500/5",
    ring: "border-blue-500/60 bg-blue-500/5 text-blue-600",
    badge: "bg-blue-50 text-blue-600 dark:bg-blue-300/10 dark:text-blue-300",
  },
  list: {
    label: "List",
    icon: LeftToRightListBulletIcon,
    overlay: "border-emerald-500/70 bg-emerald-500/10",
    mutedOverlay: "border-emerald-500/35 bg-emerald-500/5",
    ring: "border-emerald-500/60 bg-emerald-500/5 text-emerald-600",
    badge:
      "bg-emerald-50 text-emerald-600 dark:bg-emerald-300/10 dark:text-emerald-300",
  },
  table: {
    label: "Table",
    icon: Table01Icon,
    overlay: "border-amber-500/70 bg-amber-500/10",
    mutedOverlay: "border-amber-500/35 bg-amber-500/5",
    ring: "border-amber-500/60 bg-amber-500/5 text-amber-700",
    badge:
      "bg-amber-50 text-amber-600 dark:bg-amber-300/10 dark:text-amber-300",
  },
  figure: {
    label: "Figure",
    icon: ImageCompositionIcon,
    overlay: "border-rose-500/70 bg-rose-500/10",
    mutedOverlay: "border-rose-500/35 bg-rose-500/5",
    ring: "border-rose-500/60 bg-rose-500/5 text-rose-600",
    badge: "bg-rose-50 text-rose-600 dark:bg-rose-300/10 dark:text-rose-300",
  },
  header: {
    label: "Header",
    icon: TextCenterlineCenterTopIcon,
    overlay: "border-cyan-500/70 bg-cyan-500/10",
    mutedOverlay: "border-cyan-500/35 bg-cyan-500/5",
    ring: "border-cyan-500/60 bg-cyan-500/5 text-cyan-700",
    badge: "bg-cyan-50 text-cyan-600 dark:bg-cyan-300/10 dark:text-cyan-300",
  },
  footer: {
    label: "Footer",
    icon: AlignBoxBottomCenterIcon,
    overlay: "border-slate-500/70 bg-slate-500/10",
    mutedOverlay: "border-slate-500/35 bg-slate-500/5",
    ring: "border-slate-500/60 bg-slate-500/5 text-slate-700",
    badge:
      "bg-slate-50 text-slate-600 dark:bg-slate-300/10 dark:text-slate-300",
  },
  page_number: {
    label: "Page number",
    icon: TextNumberSignIcon,
    overlay: "border-zinc-500/70 bg-zinc-500/10",
    mutedOverlay: "border-zinc-500/35 bg-zinc-500/5",
    ring: "border-zinc-500/60 bg-zinc-500/5 text-zinc-700",
    badge: "bg-zinc-50 text-zinc-600 dark:bg-zinc-300/10 dark:text-zinc-300",
  },
}

function getBlockType(block: ParsedOcrBlock): OcrBlockType | undefined {
  if (block.type === "heading" || block.type === "section_heading") {
    return "heading"
  }

  if (block.type === "header") {
    return "header"
  }

  if (block.type === "footer") {
    return "footer"
  }

  if (block.type === "page_number") {
    return "page_number"
  }

  if (block.type === "figure" || block.type === "image") {
    return "figure"
  }

  if (block.type === "table") {
    return "table"
  }

  if (block.metadata.layoutClass === "List Item") {
    return "list"
  }

  if (block.type === "text") {
    return "paragraph"
  }
}

export function getOcrBlocks(output: ParsedOcrOutput): OcrBlock[] {
  return output.chunks.flatMap((chunk) =>
    chunk.blocks.flatMap((block) => {
      const type = getBlockType(block)
      const { page } = block.metadata

      if (!type || page.width <= 0 || page.height <= 0) {
        return []
      }

      return {
        id: block.id,
        type,
        text: block.content,
        page: page.number,
        pageWidth: page.width,
        pageHeight: page.height,
        confidence:
          block.metadata.avgOcrConfidence ??
          block.metadata.minOcrConfidence ??
          1,
        polygon: block.polygon,
        boundingBox: block.boundingBox,
      }
    })
  )
}

function getBoundingBox(block: OcrBlock): BoundingBox {
  if (block.boundingBox) {
    return block.boundingBox
  }

  const polygon = block.polygon ?? []
  const xValues = polygon.map((point) => point.x)
  const yValues = polygon.map((point) => point.y)
  const left = Math.min(...xValues)
  const right = Math.max(...xValues)
  const top = Math.min(...yValues)
  const bottom = Math.max(...yValues)

  return { left, top, right, bottom }
}

function getBlockCoordinateRotation(
  block: OcrBlock,
  pageSize?: { width: number; height: number }
) {
  const blockIsLandscape = block.pageWidth > block.pageHeight

  if (!pageSize) return blockIsLandscape ? 1 : 0

  const pageIsLandscape = pageSize.width > pageSize.height

  if (blockIsLandscape && !pageIsLandscape) return 1
  if (!blockIsLandscape && pageIsLandscape) return 3

  return 0
}

function normalizeHighlightAreaForRotation(
  area: HighlightArea,
  rotation: number
): HighlightArea {
  if (rotation === 1) {
    return {
      left: area.top,
      top: 100 - area.left - area.width,
      width: area.height,
      height: area.width,
    }
  }

  if (rotation === 3) {
    return {
      left: 100 - area.top - area.height,
      top: area.left,
      width: area.height,
      height: area.width,
    }
  }

  return area
}

export function blockToHighlightArea(
  block: OcrBlock,
  pageSize?: { width: number; height: number }
): HighlightArea {
  const { left, top, right, bottom } = getBoundingBox(block)

  const area = {
    left: (left / block.pageWidth) * 100,
    top: (top / block.pageHeight) * 100,
    width: ((right - left) / block.pageWidth) * 100,
    height: ((bottom - top) / block.pageHeight) * 100,
  }

  return normalizeHighlightAreaForRotation(
    area,
    getBlockCoordinateRotation(block, pageSize)
  )
}

export function blockToArea(
  block: OcrBlock,
  pageSize?: { width: number; height: number }
): React.CSSProperties {
  const area = blockToHighlightArea(block, pageSize)

  return {
    left: `${area.left}%`,
    top: `${area.top}%`,
    width: `${area.width}%`,
    height: `${area.height}%`,
  }
}

const OcrBlockMarkdown = React.memo(function OcrBlockMarkdown({
  text,
}: {
  text: string
}) {
  const markdown = text.replace(OCR_MARKDOWN_FIGURE_CAPTION_PATTERN, (tag) =>
    tag.startsWith("</") ? "</figcaption>" : "<figcaption>"
  )

  return (
    <div className="space-y-1 text-sm leading-5 text-foreground/90">
      <ReactMarkdown
        rehypePlugins={OCR_MARKDOWN_REHYPE_PLUGINS}
        remarkPlugins={OCR_MARKDOWN_REMARK_PLUGINS}
        components={{
          h1: ({ node: _node, ...props }) => (
            <h1
              className="my-0 text-base leading-5 font-semibold text-foreground"
              {...props}
            />
          ),
          p: ({ node: _node, ...props }) => (
            <p className="my-0 text-[13px] leading-5" {...props} />
          ),
          ol: ({ node: _node, ...props }) => (
            <ol className="my-0 list-decimal space-y-0.5 pl-4" {...props} />
          ),
          table: ({ node: _node, ...props }) => (
            <div className="overflow-hidden rounded-md border bg-background">
              <table className="w-full border-collapse text-xs" {...props} />
            </div>
          ),
          figure: ({ node: _node, ...props }) => (
            <figure
              className="my-0 space-y-2 rounded-md border bg-muted/20 p-2 text-[13px]"
              {...props}
            />
          ),
          figcaption: ({ node: _node, ...props }) => (
            <figcaption
              className="border-t pt-2 text-xs leading-5 text-muted-foreground"
              {...props}
            />
          ),
          caption: ({ node: _node, ...props }) => (
            <figcaption
              className="block border-t pt-2 text-xs leading-5 text-muted-foreground"
              {...props}
            />
          ),
          th: ({ node: _node, ...props }) => (
            <th className="border-b bg-muted px-2 py-1 text-left" {...props} />
          ),
          td: ({ node: _node, ...props }) => (
            <td className="border-t px-2 py-1" {...props} />
          ),
        }}
      >
        {markdown}
      </ReactMarkdown>
    </div>
  )
})

const OcrBlockButton = React.memo(function OcrBlockButton({
  block,
  isActive,
  onFocusBlock,
}: {
  block: OcrBlock
  isActive: boolean
  onFocusBlock: (block: OcrBlock) => void
}) {
  const style = BLOCK_STYLES[block.type]

  return (
    <button
      type="button"
      onMouseEnter={() => onFocusBlock(block)}
      onFocus={() => onFocusBlock(block)}
      className={cn(
        "w-full rounded-lg border bg-background p-3 text-left hover:bg-muted/40 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none",
        isActive && style.ring
      )}
    >
      <div className="min-w-0">
        <div className="flex items-center justify-between gap-2">
          <div className="flex min-w-0 items-center gap-2">
            <div
              className={cn(
                "inline-flex shrink-0 items-center gap-1 rounded-full px-2 py-0.5 text-xs font-medium",
                style.badge
              )}
            >
              <HugeiconsIcon icon={style.icon} className="size-3.5" />
              {style.label}
            </div>
            <div className="truncate text-xs text-muted-foreground">
              {Math.round(block.confidence * 100)}%
            </div>
          </div>
          <div className="shrink-0 rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">
            p. {block.page}
          </div>
        </div>
        <div className="mt-2 text-sm text-foreground/90">
          <OcrBlockMarkdown text={block.text} />
        </div>
      </div>
    </button>
  )
})

export const OcrBlockOverlay = React.memo(function OcrBlockOverlay({
  block,
  isActive,
  pageHeight,
  pageWidth,
}: {
  block: OcrBlock
  isActive?: boolean
  pageHeight?: number
  pageWidth?: number
}) {
  const style = BLOCK_STYLES[block.type]

  return (
    <div
      className={cn(
        "pointer-events-none absolute z-10 border",
        isActive ? style.overlay : style.mutedOverlay
      )}
      style={blockToArea(
        block,
        pageWidth && pageHeight
          ? { width: pageWidth, height: pageHeight }
          : undefined
      )}
    />
  )
})

export function OcrBlocksPanel({
  activeBlockId,
  blocks,
  className,
  onBlockFocus,
}: {
  activeBlockId?: string
  blocks: OcrBlock[]
  className?: string
  onBlockFocus?: (block: OcrBlock) => void
}) {
  const scrollViewportRef = React.useRef<HTMLDivElement | null>(null)
  const [localActiveBlockId, setLocalActiveBlockId] = React.useState(
    activeBlockId ?? blocks[0]?.id
  )
  const firstBlock = blocks[0]
  const focusedBlockId = activeBlockId ?? localActiveBlockId
  const activeBlock =
    blocks.find((block) => block.id === focusedBlockId) ?? firstBlock
  const focusedBlockIdRef = React.useRef(focusedBlockId)

  React.useEffect(() => {
    focusedBlockIdRef.current = focusedBlockId
  }, [focusedBlockId])

  const estimateBlockSize = React.useCallback(
    (index: number) => {
      const block = blocks[index]
      return block
        ? getEstimatedOcrBlockRowHeight(block)
        : OCR_BLOCK_ROW_MIN_ESTIMATE
    },
    [blocks]
  )
  const virtualizer = useVirtualizer({
    count: blocks.length,
    estimateSize: estimateBlockSize,
    getItemKey: (index) => blocks[index]?.id ?? index,
    getScrollElement: () => scrollViewportRef.current,
    overscan: 6,
  })

  const focusBlock = React.useCallback(
    (block: OcrBlock) => {
      if (block.id === focusedBlockIdRef.current) return

      focusedBlockIdRef.current = block.id
      setLocalActiveBlockId(block.id)
      onBlockFocus?.(block)
    },
    [onBlockFocus]
  )

  React.useEffect(() => {
    if (!firstBlock) return
    if (
      activeBlockId ||
      blocks.some((block) => block.id === localActiveBlockId)
    ) {
      return
    }

    setLocalActiveBlockId(firstBlock.id)
  }, [activeBlockId, blocks, firstBlock, localActiveBlockId])

  return (
    <aside
      className={cn("flex h-[420px] min-h-0 flex-col bg-background", className)}
    >
      <ScrollArea
        className="min-h-0 flex-1"
        scrollFade
        viewportRef={scrollViewportRef}
      >
        {blocks.length ? (
          <div
            className="relative"
            style={{
              height: virtualizer.getTotalSize() + OCR_BLOCK_LIST_PADDING * 2,
            }}
          >
            {virtualizer.getVirtualItems().map((virtualRow) => {
              const block = blocks[virtualRow.index]
              if (!block) return null

              return (
                <div
                  key={virtualRow.key}
                  ref={virtualizer.measureElement}
                  data-index={virtualRow.index}
                  className="absolute top-0 right-3 left-3 pb-2 [contain:layout_paint]"
                  style={{
                    transform: `translateY(${
                      virtualRow.start + OCR_BLOCK_LIST_PADDING
                    }px)`,
                  }}
                >
                  <OcrBlockButton
                    block={block}
                    isActive={block.id === activeBlock?.id}
                    onFocusBlock={focusBlock}
                  />
                </div>
              )
            })}
          </div>
        ) : (
          <div className="p-3">
            <div className="rounded-lg border bg-muted/30 p-3 text-sm text-muted-foreground">
              No layout blocks found.
            </div>
          </div>
        )}
      </ScrollArea>
    </aside>
  )
}

export function OcrBlocks() {
  return (
    <OcrBlocksPanel blocks={getOcrBlocks(ATTENTION_OCR_OUTPUT).slice(0, 12)} />
  )
}
