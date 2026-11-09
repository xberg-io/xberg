| Distribution of Bounding Boxes per Frame |  | Distribution of Bounding Boxes per Polyp |
| --- | --- | --- |
| 2415614 334047 7098 835 of Frames (log scale) 129 | 25 20 15 10 5 0 | Number of Polyps |

6

4

3

2 0 1 2 3 4 10<sup>2</sup> 10<sup>3</sup> 10<sup>4</sup> 10<sup>5</sup>

Number of Boxes per Frame Number of Boxes per Polyp (log scale)

**Figure 4.** Left, a histogram displaying the number of boxes per frame. On the right, the distribution of the number of bounding boxes associated to each polyp.

of detection, tracking, and classification tasks in colonoscopy compared to standard computer vision tasks that often involve simultaneous detection of numerous classes or objects, as seen in MS COCO or Kinetics datasets. Figure 4 (right) illustrates the distribution of bounding boxes per polyp, indicating a mean appearance duration of 1 minute and 15 seconds per polyp, with the longest enduring over 12 minutes. The bounding box detections corresponding to a single polyp are not necessarily contiguous. They may fracture into numerous tracklets as polyps can momentarily disappear from view. This phenomenon is elucidated on the left side of Figure 5, which showcases a histogram of the number of tracklets into which the dataset polyps are divided. In this figure, the criterion for identifying a new tracklet is the absence of a bounding box instance for that polyp for more than one second. It can be noticed that utilizing this criterion, only 10 out of 132 polyps have a continuous appearance without disappearing from view for more than 1 second. This underlines the relevance of the task of polyp tracking for real-time colonoscopy applications where accurately re-identify tracklets is essential to prevent the need to restart temporal analysis with each polyp disappearance. Furthermore, the right side of Figure 5 presents the total number of tracklets that emerge when applying different temporal thresholds. For instance, when applying a 15-second threshold, over 100 tracklets persist, possibly signifying the most challenging cases. Figure 6 (right) presents two heatmaps illustrating the spatial distribution of polyps within the frames, stratified by their time of appearance: the initial second and thereafter. The heatmaps emphasize that endoscopists tend to frame polyps closer to the center of the image as time progresses, despite their initial scattered appearance. In contrast, Figure 6 (left) exhibits two boxplots contrasting the bounding box dimensions over time with the actual sizes (in millimeters) as indicated in the dataset. These plots reveal that irrespective of their actual sizes, all bounding boxes initially have smaller dimensions during early detection. Additionally, it can be observed that as detection progresses, all polyps are characterized by numerous bounding boxes occupying more than 50% of the image. This indicates that determining polyp size solely based on bounding box dimensions is challenging and requires additional reasoning on contextual spatial-temporal information.

**Polyp Detection**

This section presents a technical validation conducted to evaluate polyp detection in the REAL-Colon dataset, primarily focusing on verifying the quality and usefulness of the dataset. For this task, we leveraged an off-the-shelf SSD object detection

38

model , as implemented in the NVIDIA's DeepLearningExamples repository. Comprehensive training and testing code is accessible via our GitHub repositories at https://github.com/cosmoimd/DeepLearningExamples. Our experiments do not merely target the absolute performance of the proposed method on the REAL-Colon dataset; instead, we focus on evaluating the dataset's utility for this specific task. Specifically, we aimed to ascertain the influence of possessing a substantial sample set for each polyp on the accuracy of the trained model. Moreover, we conducted multiple training experiments incorporating varying proportions of negative frames from each video (1%, 5%, 10 % and 100 % of negative frames from each video) in the training dataset to evaluate their impact on model performance. We partitioned the first 10 videos from each cohort into the training set, the following two videos into the validation set,

**7/12**
