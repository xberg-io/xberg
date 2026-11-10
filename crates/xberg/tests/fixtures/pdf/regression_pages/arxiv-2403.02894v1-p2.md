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

*s t* 2πβ *ft* + sin π *ft*

( )<sup>=</sup>

exp(<sup>*j*</sup>

###### ( <sup>2</sup> )

)

## (<sup>*C*</sup> )

wbi

#### ∑ n

*cn n*

- A. *RFI signal model* Common RFI can be categorized into narrowband interference, chirp broadband interference and sinusoidal = broadband interference [6]. The narrowband interference can be expressed as:

*N*

sm *n* =1

 (4) *J* sin ππ *f t* + 2 *mf t*

###### ( <sup>β</sup>) (<sup>2</sup> )

*Nm* =∞ 2 *mn c n*

 *C*~*n*~

#### ∑∑~*nm*~ ~=1 =−∞~++ *J* cos ππ *f t* 2 *mf t*

###### ( <sup>β</sup>) (<sup>2</sup> )

2 *mn c n*



*mk* +2 *k* ∞

( <sup>−</sup>1)

β Where, *J*

( <sup>β</sup> )<sup>=</sup>

*s A* exp + *jf* 2πτ (1) . According to

##### (<sup>τ</sup>)<sup>=</sup> (<sup>*jf*</sup> 2πτ )

2 *mn*∑~*k*~ =0 *kmk* \!   *n* nbi

## ∑ n

*cn n* =1

(

\+

##### )<sup>\!2</sup>

Where, *f*~*c*~ is the carrier frequency, *A*~*n*~ is the amplitude, *f*~*n*~ is (4), sinusoidally modulated broadband interference is the frequency offset, and *N* is the number of interference composed of a series of narrowband interferences, signals. approximating a specific form of chirp modulated broadband The chirp modulation interference can be expressed as: interference.

*N*

Formulas (1), (2), (4) can be uniformly expressed as: *s B* exp (2)

(<sup>τ</sup> )<sup>=</sup>

# (<sup>*j*</sup> 2πτ *f* ~*cn*~+ *jk* πτ2)

wbi~cm~

## ∑~*n*~ =1 *n*

*N*

*s D* exp *j k* (5)

( <sup>*t*</sup>)<sup>=</sup>

# (<sup>*j*</sup> 2ππ *f*~*c*~*t* + *RFIt* 2)

*RFI*

## ∑ n

Where, *B*~*n*~ is the amplitude, and *k*~*n*~ is the chirp rate.

*n* =1

Where, *D*~*n*~ is the amplitude, *k*~*RFI*~is the tuning rate. When *k* The sinusoidal modulation interference can be represented ~*RFI*~ as: is small, RFI is a narrowband interference, and when *k*~*RFI*~ is

*N*

*s C* exp πβ *ft* + sin *ft* large, RFI is a broadband interference.

( <sup>*t*</sup>)<sup>=</sup>

### (<sup>*j*</sup> 2

( <sup>2</sup>π)) <sup>(3)</sup>

wbi~sm~

## ∑ n

*cn n n* =1

By conducting time-frequency analysis on equation (5), it Where, *C*~*n*~ is the amplitude, β is the modulation coefficient ~*n*~ can be found that: and *f*~*n*~ is the modulation frequency. According to the Bessel *fk*~*RFI*~ = ⋅ *t* (6) formula, sinusoidally modulated broadband interference can From formula (6), it can be seen that the signal holds global be expressed as: characteristics on the time-frequency map, which inspires us to propose a Transformer network. Moreover, from the interference model in Equation (5), it can be seen that the
