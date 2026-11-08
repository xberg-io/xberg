| Sex |  | Age Group |  | Polyp Count |
| --- | --- | --- | --- | --- |
| 27 (45.0%) | (40, 50] (50, 60] (60, 70] Age (70, 80] 33 (55.0%) (80, 90] | 3 (5.1%) 18 15 (25.4%) 3 (5.1%) | 0 1 2 20 (33.9%) 3 4 (30.5%) 5 6 7 8 9 | 14 (23.3%) 6 (10.0%) 10 (16.7%) 5 (8.3%) 2 (3.3%) 3 (5.0%) Number of Polyps 1 (1.7%) 1 (1.7%) 1 (1.7%) |

17 (28.3%) Female

Sex

Male

0 10 20 30 0 5 10 15 20 0 5 10 15 Number of Subjects Number of Subjects Number of Subjects Bowel Cleansing Endoscope Brand Procedure Duration

6 3 (5.0%) (10, 30] 40 (66.7%)

Fujifilm 7 (11.7%)

7 9 (15.0%) (30, 50] 13 (21.7%)

BBPS score

8 16 (26.7%)

Duration [min]

(50, 70] 4 (6.7%)

Endoscope Brand

Olympus 53 (88.3%)

9 32 (53.3%) (70, 90] 3 (5.0%)

0 10 20 30 0 10 20 30 40 50 0 10 20 30 40 Number of Subjects Number of Subjects Number of Subjects

**Figure 2.** Clinical Data Distribution. This figure presents histograms depicting the distribution of sex, age, polyp count per procedure, BBPS scores, endoscope brand, and procedure duration within the REAL-Colon dataset.

The dataset encompasses a broad range of patients, with an average age of 64.6±10.7 and a male predominance of 55%. Notably, 14 out of the 60 videos (representing 23%) within the dataset do not contain any polyps, which underscores the variety of clinical scenarios captured. For a comprehensive understanding and visualization of the distribution of these variables within our dataset, we direct readers to Figure 2. This figure presents histograms detailing these parameters, thereby providing a more holistic view of the dataset.

**Polyp Detection Annotation**

A team of ten medical image annotation specialists executed a comprehensive bounding box annotation protocol under the supervision of expert gastroenterologists. A key aspect of the protocol involved annotating the polyp, beginning from its total resection and proceeding in reverse frame by frame until its initial appearance. This approach facilitated lesion tracking across sequential frames, even when the lesions were temporarily invisible. To accomplish the task, the team employed a specialized, in-house annotation tool from Cosmo Intelligent Medical Devices. Although not available for public use, this tool mirrors the functionalities of many other video annotation software, enabling the sequential identification of polyps and the application of bounding boxes around them within each frame. To ensure accuracy and consistency, the polyp annotation process was iterative, with annotations undergoing refinements through weekly meetings. These sessions provided a platform for collaboration, promoting valuable exchanges between the annotation team and the supervising expert gastroenterologists. This procedure culminated in a dataset comprising 2,757,723 frames and 132 excised colorectal polyps. The process yielded a total of 351,264 bounding box annotations.

**Polyp Histopathological Information**

Each annotation of polyps underwent cross-verification against its corresponding eCRF for each patient. The objective of this validation process, supervised by an expert gastroenterologist, was to eliminate any potential discrepancies between the video

**5/12**
