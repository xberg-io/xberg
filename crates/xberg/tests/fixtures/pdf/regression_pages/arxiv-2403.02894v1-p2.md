and the proposed method achieves better interference imaging process. The overall process is illustrated in Fig. 1, suppression effect. Compared with image inpainting networks, and the algorithm is shown in Algorithm I. this method holds stronger generalization ability in crosssensor experiments. Even if the training data and testing data come from different sensors, the algorithm can still achieve excellent results. What's more, our method only requires the interfered data to perform interference suppression. Therefore, this approach can bypass the issue of incomplete data. The organization of this paper is as follows: Section 2 introduces the signal model and the network. Section 3 presents the experimental results. And section 4 provides a summary of the entire paper.

- II. METHOD

This paper proposes a network based on domain invariant features (DIFNet), and embeds the RFI suppression into SAR

Algorithm I: DIFNet's pipeline

1. Detect RFI in SAR images;
2. If there are SAR echoes: Locate interference echoes; else: Convert interfered images into echoes;
3. Perform STFT pulse-by-pulse;
4. Predict RFI by DIFNet;
5. Subtract RFI;
6. Perform ISTFT pulse-by-pulse;
7. Convert SAR echoes into SAR images.

**Fig. 1.** Flow chart of RFI suppression network based on domain invariant features.

*N*

*s ft* + sin *ft*

( <sup>*t*</sup>)<sup>=</sup>

#### (<sup>*j*</sup> 2πβ( <sup>2</sup>π))

## (<sup>*C*</sup> exp )

wbi~sm~

#### ∑ n

*cn n*

- A. *RFI signal model*

*n* =1

Common RFI can be categorized into narrowband  *J f t* + 2 *mf t* (4)

###### ( <sup>β</sup>)<sup>sin</sup> (<sup>2</sup>ππ)

*Nm* =∞ 2 *mn c n*

=  *C* interference, chirp broadband interference and sinusoidal ~*n*~

#### ∑∑~*nm*~ ~=1 =−∞~

\++ *J f t* 2 *mf t*

###### ( <sup>β</sup>) <sup>cos</sup>(<sup>2</sup>ππ)

2 *mn c n*

broadband interference [6]. 

*mk* +2

The narrowband interference can be expressed as:

*k* ∞

( <sup>−</sup>1)

β

*N n*

Where, *J*

( <sup>β</sup> )<sup>=</sup>

*s* exp + *jf* 2πτ (1) . According to

##### (<sup>τ</sup>)<sup>=</sup> (<sup>*jf*</sup> 2πτ )

2 *mn*∑



nbi

∑

*A*~*n*~

*cn k* =0 *n* =1

*kmk* \!( +

)<sup>\!2</sup>

Where, *f*~*c*~ is the carrier frequency, *A*~*n*~ is the amplitude, *f*~*n*~ is (4), sinusoidally modulated broadband interference is the frequency offset, and *N* is the number of interference composed of a series of narrowband interferences, signals. approximating a specific form of chirp modulated broadband The chirp modulation interference can be expressed as: interference.

*N* 2

Formulas (1), (2), (4) can be uniformly expressed as: *s* exp *f* + *jk* πτ (2)

(<sup>τ</sup> )<sup>=</sup>

# (<sup>*j*</sup> 2πτ )

wbi~cm~

∑

*B*~*n*~

*cn n* =1 *N* 2

*s* exp *ft* + *j k t* (5)

( <sup>*t*</sup>)<sup>=</sup>

# (<sup>*j*</sup> 2ππ)

*RFI*

∑

*D*~*n*~

*c RFI*

Where, *B*~*n*~ is the amplitude, and *k*~*n*~ is the chirp rate.

*n* =1

Where, *D*~*n*~ is the amplitude, *k*~*RFI*~is the tuning rate. When *k* The sinusoidal modulation interference can be represented ~*RFI*~ as: is small, RFI is a narrowband interference, and when *k*~*RFI*~ is

*N*

*s* exp *ft* + sin *ft* large, RFI is a broadband interference.

( <sup>*t*</sup>)<sup>=</sup>

### (<sup>*j*</sup> 2πβ( <sup>2</sup>π)) <sup>(3)</sup>

wbi~sm~

∑

*C*~*n*~

*cn n n* =1

By conducting time-frequency analysis on equation (5), it Where, *C*~*n*~ is the amplitude, β is the modulation coefficient ~*n*~ can be found that: and *f*~*n*~ is the modulation frequency. According to the Bessel *fk*~*RFI*~ = ⋅ *t* (6) formula, sinusoidally modulated broadband interference can From formula (6), it can be seen that the signal holds global be expressed as: characteristics on the time-frequency map, which inspires us to propose a Transformer network. Moreover, from the interference model in Equation (5), it can be seen that the
