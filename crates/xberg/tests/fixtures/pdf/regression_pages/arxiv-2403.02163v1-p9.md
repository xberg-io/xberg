| Num Frames per Lesion | Ratio of Neg Frames | \#Train Images | AP | AP50 | AP75 | FPR | TPR |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 100 | 0 | 11141 | 0.107 | 0.181 | 0.117 | 0.123 | 0.463 |
| 1000 | 0 | 107525 | 0.137 | 0.231 | 0.147 | 0.082 | 0.464 |
| all | 0 | 230936 | 0.165 | 0.272 | 0.181 | 0.080 | 0.495 |
| all | 0.01 | 246556 | 0.190 | 0.286 | 0.217 | 0.058 | 0.434 |
| all | 0.05 | 309116 | 0.193 | 0.311 | 0.219 | 0.072 | 0.472 |
| all | 0.1 | 387315 | 0.178 | 0.265 | 0.205 | 0.083 | 0.454 |
| all | all | 1794907 | 0.216 | 0.338 | 0.245 | 0.054 | 0.505 |

**Table 3.** This table presents detection results from experiments utilizing different training dataset subsets, with metrics aligned to COCO object detection. AP measures the mean Average Precision across a range of IoU thresholds from 0.5 to 0.95, while AP50 and AP75 specifically refer to IoU thresholds of 0.5 and 0.75, respectively. The False Positive Rate (FPR) and True Positive Rate (TPR) indicate the percentage of polyp-negative frames in which models erroneously detected bounding boxes and the percentage of positive frames in which models flagged an alert, respectively. Testing was conducted on the entire 591,647 frames test set. Each row corresponds to a distinct subset of training images from the 40 videos in the training split, varying the number of selected positive frames per lesion and the ratio of selected negative frames to total negative frames per video.

| Polyp | \#Test Images | \#Test Polyps | AP | AR |
| --- | --- | --- | --- | --- |
| Diminutive | 52,133 | 17 | 0.347 | 0.450 |
| Non-Diminutive | 29,973 | 4 | 0.120 | 0.260 |
| Adenoma | 37,264 | 12 | 0.427 | 0.498 |
| Non-Adenoma | 40,525 | 8 | 0.145 | 0.295 |
| Hyperplastic Sigma-Rectum | 10,217 | 5 | 0.387 | 0.505 |
| Others | 71,889 | 16 | 0.260 | 0.364 |
| All | 82,106 | 21 | 0.277 | 0.381 |

**Table 4.** Average Precision and Average Recall (AR) for IoU=0.5 to 0.95 on different subsets of positive polyp frames within the test set. The data is segmented to detail performance distinctions among different subsets of the training set alongside a comprehensive assessment for all the polyp positive test images.

| Appearance | \#Test Images | AP | AR |
| --- | --- | --- | --- |
| \< 1s | 570 | 0.138 | 0.254 |
| \< 3s | 1,710 | 0.179 | 0.334 |
| All | 82,106 | 0.217 | 0.381 |

**Table 5.** Performance of the best SSD detection model on frames from the initial second and first three seconds of polyp appearance, rather than evaluating all positive frames.

and the remaining three videos into the test set. For each model training session, we used a batch size of 96 and an image resolution of 300x300. Standard augmentation techniques, including scaling, random cropping, horizontal flipping, and image normalization, were applied, using the same parameters as those utilized for the MS COCO Dataset in the repository. All models were trained and tested on an Nvidia Tesla V100 GPU. In Table 3, we report performance in terms of Average Precision (AP), measuring the mean AP across a range of IoU thresholds from 0.5 to 0.95 (measuring overlap between model bounding box predictions and GTs), in steps of 0.05. AP50 and AP75, instead, specifically refer to the model Average Precision at IoU thresholds of 0.5 and 0.75, respectively, and indicate more accurate detection performance. For every model, we also the False Positive Rate (FPR) and the True Positive Rate (TPR) per video, defined as the percentage of negative frames in which models erroneously detected bounding boxes, indicating the rate of false alarms on negative frames, and the percentage of positive frames in which at least a bounding box was detected. Throughout the training process, the validation set was used to monitor the AP at various epochs, and the best-performing model on the validation set throughout the training was selected. All experiments were tested on the same test set of 591,647 frames, including all frames (with and without boxes) from 12 test videos. The initial analysis presented in Table 3 highlights that incorporating a greater number of positive samples, despite them representing identical polyps, significantly boosts the model's performance. Subsequent rows detail the effects of varying negative sample proportions in the training set, indicating optimal SSD model performance with the full inclusion of available negative images alongside all positive frames. Although the current strategy prioritizes this comprehensive approach, future advancements might benefit from a predefined negative-to-positive frame ratio per training epoch to refine the training process further. The FPR and TPR results also underscore accuracy improvements by fully incorporating negative frames during

**9/12**
