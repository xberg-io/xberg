and the proposed method achieves better interference imaging process. The overall process is illustrated in Fig. 1, suppression effect. Compared with image inpainting networks, and the algorithm is shown in Algorithm I. this method holds stronger generalization ability in crosssensor experiments. Even if the training data and testing data Algorithm I: DIFNet's pipeline come from different sensors, the algorithm can still achieve

1. Detect RFI in SAR images; excellent results. What's more, our method only requires the
2. If there are SAR echoes: interfered data to perform interference suppression. Therefore, Locate interference echoes; this approach can bypass the issue of incomplete data. else: The organization of this paper is as follows: Section 2 Convert interfered images into echoes; introduces the signal model and the network. Section 3
3. Perform STFT pulse-by-pulse; presents the experimental results. And section 4 provides a
4. Predict RFI by DIFNet; summary of the entire paper.
5. Subtract RFI;
6. Perform ISTFT pulse-by-pulse;

<!-- end list -->

- II. METHOD

<!-- end list -->

7. Convert SAR echoes into SAR images.

This paper proposes a network based on domain invariant features (DIFNet), and embeds the RFI suppression into SAR

**Fig. 1.** Flow chart of RFI suppression network based on domain invariant features.

*N*

- A. *RFI signal model s* ~wbi~

( <sup>*t*</sup>)<sup>=</sup>∑ (<sup>*C*</sup> ~*n*~ exp(<sup>*j*</sup> 2πβ *ft*~*cn*~ + sin ( <sup>2</sup>π *f*~*n*~*t*)))

sm *n* =1

Common RFI can be categorized into narrowband  (4)

*Nm* =∞ *J*~2~ *mn* ( <sup>β</sup>)<sup>sin</sup> (<sup>2</sup>ππ *f t*~*c*~ + 2 *mf*~*n*~ *t*)

interference, chirp broadband interference and sinusoidal =∑∑~*nm*~ ~=1~<sup></sup> *C*~*n*~ <sup>=−∞</sup> ++ *J*~2~ ~*mn*~ ( <sup>β</sup>) <sup>cos</sup>(<sup>2</sup>ππ *f t*~*c*~ 2 *mf*~*n*~ *t*) broadband interference [6].  The narrowband interference can be expressed as:

*k mk* +2 *N* ∞

( <sup>−</sup>1)

β~*n*~ *s*(<sup>τ</sup>)<sup>=</sup> *A* exp(<sup>*jf*</sup> 2πτ+ *jf* 2πτ)

- (1) Where, *J*~2~ *mn*( <sup>β</sup> )<sup>=</sup>∑

. According to

nbi

# ∑~*n*~ =1

*n cn k* =0

 *kmk* \!( +

)<sup>\!2</sup>

Where, *f*~*c*~ is the carrier frequency, *A*~*n*~ is the amplitude, *f*~*n*~ is (4), sinusoidally modulated broadband interference is the frequency offset, and *N* is the number of interference composed of a series of narrowband interferences, signals. approximating a specific form of chirp modulated broadband The chirp modulation interference can be expressed as: interference.

*N* 2

Formulas (1), (2), (4) can be uniformly expressed as: *s*~wbi~

(<sup>τ</sup> )<sup>=</sup>∑

*B*~*n*~ exp(<sup>*j*</sup> 2πτ *f* ~*cn*~+ *jk* πτ) (2)

cm *n* =1 *N* 2

*s* ~*RFI*~( <sup>*t*</sup>)<sup>=</sup>∑ *D*~*n*~exp (<sup>*j*</sup> 2ππ *f*~*c*~*t* + *j k* ~*RFI*~*t*)

- (5) Where, *B*~*n*~ is the amplitude, and *k*~*n*~ is the chirp rate.

*n* =1

The sinusoidal modulation interference can be represented Where, *D*~*n*~ is the amplitude, *k*~*RFI*~is the tuning rate. When *k*~*RFI*~ as: is small, RFI is a narrowband interference, and when *k*~*RFI*~ is

*N*

*s* ~wbi~

( <sup>*t*</sup>)<sup>=</sup>∑

*C*~*n*~ exp (<sup>*j*</sup> 2πβ *ft*~*cn*~ + sin ( <sup>2</sup>π *f*~*n*~*t*)) <sup>(3)</sup> large, RFI is a broadband interference.

sm *n* =1

By conducting time-frequency analysis on equation (5), it Where, *C*~*n*~ is the amplitude, β is the modulation coefficient ~*n*~ can be found that: and *f*~*n*~ is the modulation frequency. According to the Bessel *fk*~*RFI*~ = ⋅ *t* (6) formula, sinusoidally modulated broadband interference can From formula (6), it can be seen that the signal holds global be expressed as: characteristics on the time-frequency map, which inspires us to propose a Transformer network. Moreover, from the interference model in Equation (5), it can be seen that the
