underpins the deployment of CSI-ResNet, an advanced adaptation of the ResNet framework, proficient in pinpointing human locational indicators within CSI. ResNet, with its residual learning approach, is particularly effective in spatial feature discernment, overcoming the vanishing gradient challenge in deep neural networks. Meanwhile, GRU and LSTM networks, as discussed in [10], [23], are tailored more towards temporal data, making them less ideal for the consistent spatial traits inherent to CSI. Our enhanced ResNet model employs a bifurcated convolutional input layer, enabling distinct processing of the real and imaginary components of complex CSI data, which adeptly captures amplitude and phase nuances, reconfiguring CSI into a dual-layered framework to better analyze spatial characteristics. By concentrating on the dimensions of transmitreceive antennas, our model gains heightened sensitivity to phase variances, enriching the extraction of locational features. Utilizing CSI's full data range, extending past RSSI, markedly improves the accuracy of indoor positioning. Additionally, the integration of deep residual learning within the ResNet framework refines its capacity for detailed spatial analysis.

- C. Multi-target loss

To adapt traditional single-person indoor positioning techniques for multi-person scenarios, a key innovation lies in the optimization of the loss function. Traditional systems typically employ single-label classification methods, identifying one target category from N potential classes. The conventional loss function integrates softmax and cross-entropy, expressed as: e<sup>sp</sup> L = − log P (7) ~N~ e<sup>sj</sup>

$$
j=1
$$

Here, $s_{p}$ is the score of the target category, while $s_{j}$ represents the scores of all candidate categories. This method aims to ensure the highest score for the target category among all classes. However, multi-person indoor localization faces a unique challenge: the discrete nature of individuals in the environment leads neural networks to predict 'empty' scenarios, i.e., all potential target locations as unoccupied. This is primarily due to the prevalence of non-target (unoccupied) samples over target (occupied) samples in training data, causing a bias towards non-target predictions. In response to the inherent limitations of traditional singletarget localization systems in multi-user scenarios, our research innovates by implementing a multi-label classification paradigm. This allows individual samples to be categorized into multiple classes. A novel loss function, termed "Multi-target loss," is designed to ensure that the score of each target category surpasses the score of any non-target category. This methodology effectively recalibrates the predictive balance between target and non-target categories, crucial for accurately capturing sparsely distributed multi-person location data. The loss function is mathematically expressed as:

$$
L_{Multi}=\frac{X}{i\in P}\max(0,\gamma-s_{i})+\frac{X}{j\in N}\max(0,s_{j}-\gamma)
$$

(8)

$$
\approx\frac{X}{i\in P}\log(1+e^{\gamma-s_{i}})+\frac{X}{j\in N}\log(1+e^{sj-\gamma})
$$

Where P and N denote the sets of positive and negative classes, respectively. This innovative approach not only enhances the predictive accuracy for target positions but also diminishes the likelihood of falsely predicting unoccupied states, thereby significantly improving the robustness and precision of multi-person localization. The parameter γ acts as a critical threshold delineating presence or absence at specific locations, enabling the network to discern between target and non-target positions more sensitively. This advanced loss function enables our model to learn positional information in multi-person scenarios effectively, extending its capabilities beyond predicting a singular most likely position to identifying multiple potential occupant locations, thus substantially enhancing the accuracy and resilience of localization. Furthermore, to address the challenge of evaluating model performance in multi-target localization scenarios, where predictions may encompass several location labels, we introduce SubACC [32] as an assessment metric. SubACC is a rigorously exacting measure, typically employed to evaluate a model's proficiency in capturing interdependencies among labels. Its formulation is as follows:

X<sup>N</sup> 1 SubACC = I(ˆ$y_{n}=y_{n})$ (9) $N_{n=1}$ Where $y_{n}$ represents the actual set of position labels for the nth prediction, and $y_{n}$ˆ is the corresponding predicted set. The indicator function I yields a value of 1 if the predicted set $y_{n}$ˆ precisely aligns with the true set $y_{n}$ and 0 otherwise. This metric stringently evaluates the congruence between predicted and actual positional datasets, providing a comprehensive assessment of the model's performance in multi-label prediction scenarios.

- D. Nuclear Norm-Based Transfer Pre-Training

To enhance robustness and generalizability in diverse indoor localization scenarios through deep CSI data analysis, we implement a novel pre-training paradigm. At the heart of this methodology is the incorporation of nuclear norm regularization into the pre-training loss [35], which is:

$$
L_{Pre}=L_{Multi}+\lambda\parallel W\parallel_{*}
$$

(10) where $L_{Multi}$ is the multi-target loss mentioned in Section III-C, $\parallel W\parallel_{&#42;}$ represents the nuclear norm of the network weight matrix W, and λ is the regularization coefficient controlling the influence of the nuclear norm. In multi-person indoor localization, the model's effectiveness hinges on precise identification and differentiation of individual positions. The nuclear norm, a robust tool for low-rank representation, encourages our network to learn streamlined, low-rank features,
