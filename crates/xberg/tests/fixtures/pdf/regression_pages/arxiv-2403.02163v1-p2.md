| Dataset | Date of Publication | Format | Resolution | Annotation | Multiple Polyp Images | Non-Polyp Images |
| --- | --- | --- | --- | --- | --- | --- |
| CVC-ColonDB14 | 2012 | 300 images | 574×500 | Segmentation | No | No |
| ETIS-Larib15 | 2014 | 196 images | 1225×966 | Segmentation | Yes | No |
| CVC-ClinicDB16 | 2015 | 612 images | 384×288 | Segmentation | Yes | No |
| ASU-Mayo17 | 2015 | 18,781 images | unknown | Segmentation | No | Yes |
| CVC-ClinicVideoDB17 | 2017 | 11,954 images | 384×288 | Segmentation | No | Yes |
| CVC-PolypHD18 | 2018 | 56 images | 1920×1080 | Segmentation | Yes | No |
| Kvasir-SEG20 | 2020 | 1000 images | 1920×1072 and 332×487 | Segmentation | Yes | No |
| PICCOLO21 | 2020 | 3433 images | 854×480 and 1920×1080 | Segmentation | Yes | Yes |
| KUMC22 | 2021 | 37,899 images | various | Bounding Box | No | Yes |
| SUN23 | 2021 | 158,690 images | 1240×1080 | Bounding Box | No | Yes |
| LDPolypVideo24 | 2021 | 40,187 images | 560x480 | Bounding Box | Yes | Yes |
| PolypGen25 | 2023 | 6282 images | various | Segmentation | Yes | Yes |
| REAL-Colon | 2023 | 2,757,723 images | 1920×1080 | Bounding Box | Yes | Yes |

**Table 1.** Available datasets for polyp detection for open research. Datasets annotated with segmentation can also be utilized to derive bounding box annotations.

| Dataset | Date of Publication | Format | Resolution | Histopathology | Size | Multiple Polyp | Non-Polyp |
| --- | --- | --- | --- | --- | --- | --- | --- |
| UAH19 | 2016 | Videoclips with Video-Label | 768×566 | SLL/Hyp/Ade | No | No | No |
| PICCOLO21 | 2020 | Images with BB and Label | 854×480 and 1920×1080 | SLL/Hyp/Ade | Yes | Yes | No |
| KUMC22 | 2021 | Videoclips with BB and Label | various | Hyp/Ade | No | No | Yes |
| SUN23 | 2021 | Videoclips with BB and Label | 1240×1080 | Complete | Yes | No | Yes |
| REAL-Colon | 2023 | Full-Procedure with BB and Label | 1920×1080 | Complete | Yes | Yes | Yes |

**Table 2.** Available datasets for polyp characterization and sizing for open research.

colonoscopy videos features 80-90% negative frames (as also illustrated in Figure 4), which are important for realistic AI model

23, 26

benchmarking and training as outlined by recent literature . Furthermore, video frames from public datasets are seldom not at native spatial-temporal resolution and typically sourced from a limited number of centers. Physicians do not perform tasks such as polyp detection and classification in the real world by statically evaluating still

7, 27

or nearly perfect images or short video clips but instead via a process of temporal visual information reasoning . The discrepancy between the available datasets and the real-world scenario inevitably affects both the design and development of CAD(e/x) algorithms, with a large part of academic research works to date still focusing on frame-by-frame approaches placing little emphasis on live processing speed and latency or full-procedure evaluation. Thus resulting in sub-optimal learning and

13, 23, 26, 28 18, 28–32

unrealistic AI model performance assessments . Similarly, open research challenges , primarily centered on the accuracy of polyp detection, segmentation, and classification tasks, have gradually shifted their focus towards enhancing model robustness, speed and efficiency. However these challenges, employing datasets detailed in Table 1 and Table 2, inherently reflect the above mentioned limitations by not fully capturing the dynamic and complex nature of real-world colonoscopy procedures. This paper aims to bridge the gap between open and privately-funded research by introducing a comprehensive, multi-center dataset of unaltered, real-world colonoscopy videos. The REAL-Colon (Real-world multi-center Endoscopy Annotated video Library) dataset comprises recordings of sixty colonoscopies. In creating this dataset, a consortium comprising Sano Hospital in Hyogo, Japan; University Hospital of St. Pölten, Austria; and Ospedale Nuovo Regina Margherita in Rome, Italy, each provided a subset of fifteen patients. These patient cases were drawn from distinct clinical studies wherein colonoscopy procedures were documented on video as part of the study protocol. Additionally, Cosmo Intelligent Medical Devices contributed by supplementing the dataset with fifteen patient cases from one of their sponsored studies conducted within the United States. Crucially, Cosmo Intelligent Medical Devices also annotated the entire dataset of 2.7M image frames to the highest quality standard. For each video, each colorectal polyp has been annotated with a bounding box in each video frame where it appeared by trained medical image annotations specialists, supervised by expert gastroenterologists to ensure their accuracy and consistency. Polyp information, including histology, size, and anatomical site, has been recorded, double-checked by annotation specialists and at least an experienced gastroenterologists, and reported with several other clinical variables. Patient variables obtained from electronic case report forms (eCRF), including endoscope brand and bowel cleanliness score (BBPS) have also been collected for each video. As illustrated in Tables 1 and 2, the REAL-Colon dataset stands unparalleled in its scale, quality, and diversity, offering a singular asset for researchers and developers dedicated to pushing the boundaries of AI in colonoscopy. Furthermore, REAL-Colon uniquely enables comprehensive benchmarking of polyp detection algorithms against the backdrop of authentic, unedited full-procedure videos, markedly distinguishing our contribution from existing datasets.

**2/12**
