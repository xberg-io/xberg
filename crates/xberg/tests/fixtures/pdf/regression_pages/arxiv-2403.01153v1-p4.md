underpins the deployment of CSI-ResNet, an advanced adapta-

X X tion of the ResNet framework, proficient in pinpointing human

L~Multi~ = max(0,γ − s ) + max(0, s − γ) locational indicators within CSI. ResNet, with its residual

i∈P

learning approach, is particularly effective in spatial feature X ≈ log(1 + discernment, overcoming the vanishing gradient challenge in

i∈P

deep neural networks. Meanwhile, GRU and LSTM networks, as discussed in [10], [23], are tailored more towards temporal

i j j∈N

X (8)

sj−γ

eγ−s~i~) + log(1 + e )

j∈N

Where P and N denote the sets of positive and negative data, making them less ideal for the consistent spatial traits classes, respectively. This innovative approach not only eninherent to CSI. hances the predictive accuracy for target positions but also di- Our enhanced ResNet model employs a bifurcated convolu- minishes the likelihood of falsely predicting unoccupied states, tional input layer, enabling distinct processing of the real and thereby significantly improving the robustness and precision imaginary components of complex CSI data, which adeptly of multi-person localization. The parameter γ acts as a critical captures amplitude and phase nuances, reconfiguring CSI into threshold delineating presence or absence at specific locations, a dual-layered framework to better analyze spatial charac- enabling the network to discern between target and non-target teristics. By concentrating on the dimensions of transmit- positions more sensitively. This advanced loss function enables receive antennas, our model gains heightened sensitivity to our model to learn positional information in multi-person scephase variances, enriching the extraction of locational features. narios effectively, extending its capabilities beyond predicting Utilizing CSI's full data range, extending past RSSI, markedly a singular most likely position to identifying multiple potential improves the accuracy of indoor positioning. Additionally, occupant locations, thus substantially enhancing the accuracy the integration of deep residual learning within the ResNet and resilience of localization. framework refines its capacity for detailed spatial analysis. Furthermore, to address the challenge of evaluating model performance in multi-target localization scenarios, where predictions may encompass several location labels, we introduce

- C. Multi-target loss

SubACC [32] as an assessment metric. SubACC is a rigorously exacting measure, typically employed to evaluate a model's To adapt traditional single-person indoor positioning techproficiency in capturing interdependencies among labels. Its niques for multi-person scenarios, a key innovation lies in the formulation is as follows: optimization of the loss function. Traditional systems typically employ single-label classification methods, identifying one X<sup>N</sup> 1 target category from N potential classes. The conventional SubACC = I(ˆy~n~ = y~n~) (9) N loss function integrates softmax and cross-entropy, expressed

n=1

as:

Where y~n~ represents the actual set of position labels for

e<sup>sp</sup> the nth prediction, and yˆ~n~ is the corresponding predicted L = − log P (7) ~N~ e<sup>sj</sup> set. The indicator function I yields a value of 1 if the

j=1

predicted set yˆ~n~ precisely aligns with the true set y~n~ and 0 Here, s~p~ is the score of the target category, while s~j~ represents otherwise. This metric stringently evaluates the congruence the scores of all candidate categories. This method aims to enbetween predicted and actual positional datasets, providing sure the highest score for the target category among all classes. a comprehensive assessment of the model's performance in However, multi-person indoor localization faces a unique multi-label prediction scenarios. challenge: the discrete nature of individuals in the environment

- D. Nuclear Norm-Based Transfer Pre-Training

leads neural networks to predict 'empty' scenarios, i.e., all To enhance robustness and generalizability in diverse indoor potential target locations as unoccupied. This is primarily due localization scenarios through deep CSI data analysis, we to the prevalence of non-target (unoccupied) samples over implement a novel pre-training paradigm. At the heart of this target (occupied) samples in training data, causing a bias methodology is the incorporation of nuclear norm regularizatowards non-target predictions. tion into the pre-training loss [35], which is:

In response to the inherent limitations of traditional singletarget localization systems in multi-user scenarios, our re- L~P~ re = L~Multi~ + λ ∥ W ∥ (10) ~∗~ search innovates by implementing a multi-label classification paradigm. This allows individual samples to be categorized where L~Multi~ is the multi-target loss mentioned in Section into multiple classes. A novel loss function, termed "Multi- III-C, ∥ W ∥ represents the nuclear norm of the network ~∗~ target loss," is designed to ensure that the score of each target weight matrix W, and λ is the regularization coefficient category surpasses the score of any non-target category. This controlling the influence of the nuclear norm. In multi-person methodology effectively recalibrates the predictive balance indoor localization, the model's effectiveness hinges on precise between target and non-target categories, crucial for accurately identification and differentiation of individual positions. The capturing sparsely distributed multi-person location data. The nuclear norm, a robust tool for low-rank representation, enloss function is mathematically expressed as: courages our network to learn streamlined, low-rank features,
