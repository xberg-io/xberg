network to learn domain invariant features, thereby ignoring targets. So, the loss can be expressed as:

2

*l* = (<sup>*HX*</sup> ~*pixel*~ −+ *Y*~*pixel*~) ε (13)

- III. EXPERIMENTS

To validate the effectiveness of the proposed method, experiments are conducted on both MiniSAR dataset and Sentinel-1 dataset. The resolution of MiniSAR is 0.1m, while the resolution of Sentinel-1 is 520m. All training data comes from MiniSAR, and the testing data comes from MiniSAR and Sentinel-1. The cross-sensor and cross-resolution experiments can be used to verify the generalization.

- A. *Evaluation Metrics* To reasonably evaluate the test results, this paper adopts pixel accuracy (PA), intersection over union (IoU), PSNR [18] and ME [12] as evaluation metric. PA and IoU are used to evaluate the DIFNet, and PSNR and ME is used to evaluate image quality. ME is defined as follows: *ME* = Ent ( <sup>Xˆ</sup> ) <sup>Mean</sup>( <sup>Xˆ</sup>) (14)

Ent ( <sup>Xˆ</sup>) <sup>is the entropy</sup>， Mean ( <sup>Xˆ</sup>) <sup>is the mean value. A</sup> smaller entropy indicates that the pixel values of the image are concentrated within a smaller range. A smaller mean value indicates a lower amplitude, suggesting that most of the

interference has been filtered out. Therefore, a smaller ME indicates a better filtering result.

- B. *MiniSAR* The filtering results on the time-frequency map are shown in Fig. 3. Comparing Fig. 3 (b) and Fig. 3(d), it can be observed that for the traditional notch filtering method, some strong scattering targets are mistakenly filtered out as interference, but for DIFNet, these strong scattering targets are well preserved. Where, the image restoration network PISNet achieved the best result. Fig. 4 shows the imaging results, and it can be seen that the proposed algorithm preserves more details and produces a cleaner filtering result comparing with traditional notch filter. The evaluation results are presented in TABLE I, it can be seen that the proposed method achieves a

1.6% improvement in PA, a 5.99% improvement in IoU, a

2.02dB improvement in PSNR, and a 0.05 decrease in ME compared to the traditional algorithm. PISNet still achieved the best result.

(a) (b) (c) (d) **Fig. 3.** Time frequency map; (a) Interfered time-frequency map; (b) Time frequency map by notch filtering; (c) Time frequency map by PISNet; (d) The time-frequency map by DIFNet.

- (a) (b) Result by DIFNet.

TABLE I

SIMUTATION RESULT

| Parameters | Interfered image | Notch filtering | PISNet | DIFNet |
| --- | --- | --- | --- | --- |
| PA | 83.09% | 94.95% | / | 96.55% |
| IoU | / | 74.41% | / | 80.40% |
| PSNR | / | 21.49 | 25.05 | 23.51 |
| ME | 0.24 | 0.09 | 0.04 | 0.04 |

- C. *Sentinel-1* The interfered dataset is obtained from Sentinel-1, captured in the Korean region on 2/16, 2019. The image area is cropped to a size of 512512. The time-frequency map is shown in Fig. 5, and the filtering results and performance indicators are shown in Fig. 6. In the Sentinel-1 testing datasets, the training data still comes from MiniSAR. And in this cross-sensor experiment, PISNet does not work, so we do not present its
- (c) (d) (e)

**Fig. 4.** Suppression results in MiniSAR; (a) Interfered image; (b) Label; (c) Result by notch filtered; (d) Result by PISNet; (e)

results, while our method can still suppress the interference, demonstrating its good generalization. Comparing Fig. 5(b) and Fig. 5 (c), it can be observed that the proposed method achieves a better filtering result. Similarly, comparing Fig. 6(b) and Fig. 6(c), it can be seen that low-intensity residual interference is difficult to filter out for traditional notch filter, while our method can effectively filter out the residual interference. Comparing with traditional notch filter, our method has reduced ME by 0.15.

- (a) (b) (c)

**Fig. 5.** Time-frequency map; (a) Interfered time-frequency map; (b) Notch filtering result; (c) DIFNet filtering result.
