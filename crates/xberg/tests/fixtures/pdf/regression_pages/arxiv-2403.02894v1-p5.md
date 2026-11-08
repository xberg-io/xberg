# (a) (b) (c)

**Fig. 6.** Suppression results in Sentinel-1; (a) Interfered image, *ME* =3.40; (b) Notch filtering image, *ME* =2.34; (c) DIFNet filtering image,

# ME =2.19 .

- [5] B. Zhao, et al., "Performance Improvement of Deception Jamming Against SAR Based on Minimum Condition Number," Ieee Journal of

# From the above experimental results, it can be seen that in

Selected Topics in Applied Earth Observations and Remote Sensing, the cross-sensor experiment, the image inpainting network Article vol. 10, no. 3, pp. 1039-1055, Mar 2017. does not even work, but our method can still achieve excellent [6] M. Tao, et al., "Mitigation of Radio Frequency Interference in Synthetic Aperture Radar Data: Current Status and Future Trends," Remote Sensing, results. The above results demonstrate that our method holds Review vol. 11, no. 20, Oct 2019, Art. no. 2438. good generalization.

- [7] F. Zhou, et al., "Eigensubspace-based filtering with application in narrowband interference suppression for SAR," Ieee Geoscience and Remote

## IV. CONCLUSION

Sensing Letters, Article vol. 4, no. 1, pp. 75-79, Jan 2007.

- [8] H. Yang, et al., "BSF: Block Subspace Filter for Removing Narrowband

# SAR is widely deployed as a high-resolution imaging radar,

and Wideband Radio Interference Artifacts in Single-Look Complex SAR but it is susceptible to intentional and unintentional RFIs. For Images," Ieee Transactions on Geoscience and Remote Sensing, Article image inpainting networks, although they have achieved vol. 60, 2022.

- [9] T. Bollian, et al."Adaptive Antenna Pattern Notching of Interference in

excellent results, their generalization is unclear. To address Synthetic Aperture Radar Data Using Digital Beamforming," Remote this problem, through time-frequency analysis of interference Sensing, Article vol. 11, no. 11, Jun 1 2019, Art. no. 1346. signals, we find that interference holds domain invariant

- [10] M. Tao et al., "Interference Mitigation for Synthetic Aperture Radar Data using Tensor Representation and Low-Rank Approximation," in 33rd

features between different sensors, and propose a SAR RFI General Assembly and Scientific Symposium (GASS) of the suppression network based on domain invariant features.

International-Union-of-Radio-Science (URSI), Rome, ITALY, 2020,

# Compared to traditional notch filtering methods, the proposed

2020\. method achieves better interference suppression effect.

- [11] S. Chen, et al., "Suppressive Interference Suppression for Airborne SAR Using BSS for Singular Value and Eigenvalue Decomposition Based on

# Furthermore, in the cross-sensor experiment, the training data

Information Entropy," Ieee Transactions on Geoscience and Remote and the testing dataset comes from different radars with Sensing, Article vol. 61, 2023 2023, Art. no. 5205611. different resolutions, and the image inpainting networks does

- [12] Y. Huang, et al, "A Novel Tensor Technique for Simultaneous

not work, but our method can still achieve excellent results.

Narrowband and Wideband Interference Suppression on Single-Channel The above demonstrates that our method holds good SAR System," Ieee Transactions on Geoscience and Remote Sensing, Article vol. 57, no. 12, pp. 9575-9588, Dec 2019. performance and generalization. Moreover, this method can

- [13] Y. Huang, Z. Chen, C. Wen, J. Li, X.-G. Xia, and W. Hong, "An Efficient

inspire self-supervised learning, as the segmented time- Radio Frequency Interference Mitigation Algorithm in Real Synthetic frequency map forms a masking task, which can be repaired Aperture Radar Data," Ieee Transactions on Geoscience and Remote Sensing, Article vol. 60, 2022 2022, Art. no. 5224912. by self-supervised networks.

- [14] Z. Wang et al., "Uformer: A General U-Shaped Transformer for Image Restoration," in IEEE/CVF Conference on Computer Vision and Pattern

# ACKNOWLEDGMENT

Recognition (CVPR), New Orleans, LA, 2022, pp. 17662-17672, 2022.

- [15] Z. Liu et al., "Swin Transformer: Hierarchical Vision Transformer using

# Thank you to the School of Electronic Science, National

Shifted Windows," in 18th IEEE/CVF International Conference on

# University of Defense Technology for your support of this

Computer Vision (ICCV), Electr Network, 2021, pp. 9992-10002, 2021. project.

- [16] W. Fan et al., "Interference Mitigation for Synthetic Aperture Radar Based on Deep Residual Network," Remote Sensing, Article vol. 11, no. 14, Jul 2 2019, Art. no. 1654.

# REFERENCES

- [17] Zhaoyun H, et al., "Self-supervised Learning Method for SAR
- [1] A. Reigber et al., "Very-High-Resolution Airborne Synthetic Aperture Interference Suppression Based on Abnormal Texture Perception," Radar Imaging: Signal Processing and Applications," Proceedings of the Journal of Radars, 2022, 12(1): 154-172. Ieee, Article vol. 101, no. 3, pp. 759-783, Mar 2013.
- [18] J. Shen, et al., "Learning Time-Frequency Information with Prior for SAR Radio Frequency Interference Suppression," Ieee Transactions on
- [2] S. Adeli et al., "Wetland Monitoring Using SAR Data: A Meta-Analysis and Comprehensive Review," Remote Sensing, Review vol. 12, no. 14, Geoscience and Remote Sensing, Article vol. 60, 2022 2022, Art. no. Jul 2020, Art. no. 2190. 5239716.
- [3] M. Bonano, et al., "From Previous C-Band to New X-Band SAR Systems: Assessment of the DInSAR Mapping Improvement for Deformation Time-Series Retrieval in Urban Areas," Ieee Transactions on Geoscience and Remote Sensing, Article vol. 51, no. 4, pp. 1973-1984, Apr 2013.
- [4] S. Joy, et al., "Joint Down-Range and Cross-Range RFI Suppression in Ultra-Wideband SAR," Ieee Transactions on Geoscience and Remote Sensing, Article vol. 59, no. 4, pp. 3136-3149, Apr 2021.
