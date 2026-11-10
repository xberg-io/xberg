Where H~s~(f, t) denotes the CSI measurements transmitted by static paths such as ceiling/wall reflections, termed as static CSI component. H~m~(f, t) and H~n~(f, t) represent the CSI measurements transmitted via body reflection paths and limb reflection paths, respectively. H(f, t) can also be expressed as:

iθ

H(f, t) = ∥H(f, t)∥ · e (2)

iθ

Where ∥H(f, t)∥ and e symbolize the amplitude and phase of the transmitted signal, respectively. Thus, the received CSI can be rewritten as:

jθ jθs jθm

∥ H ( f, t ) ∥

- e = ∥ H s ( f, t ) ∥
- e + ∥ H m ( f, t ) ∥
- e (3)

jθn

\+∥H~n~(f, t)∥ · e However, the phase information in CSI is often affected by factors like carrier frequency offset and sampling frequency offset, leading to significant errors in CSI phase information. Also, the amplitude information in CSI changes with device restarts. Hence, the actual measured CSI is:

jθs

∥ H R ( f, t ) ∥

- e jθ R = α ( ∥ H s ( f, t ) ∥
- e (4)

jθm jθn jθe

\+ ∥ H m ( f, t ) ∥

- e + ∥ H n ( f, t ) ∥
- e ) e Where ∥ H R ( f, t ) ∥ is the actual received amplitude, θ R is the actual received phase, α is the amplitude offset, and θ e is the intrinsic phase difference between receiving antennas. Therefore, the received CSI requires calibration.
- A. CSI preprocessing As delineated in research [24], [25], frequent rebooting of transceivers impacts oscillator configuration, leading to random phase differences between receiving antennas. For precise intrinsic phase difference acquisition between receiving antennas, we followed the connection scheme depicted in Fig. 3 for transceivers and conducted a series of detailed experimental analyses. After restarting the transceivers 20 times, the phase differences between the receiving antennas were meticulously recorded. By aggregating data across all carriers and time packets, a probability distribution was obtained, as illustrated in Fig. 4.

Fig. 3. Experiment for CSI calibration. (a) Phase Offset Measurement Experiment Setup. (b) Spliter/combiner. (c) Attenuator. (d) Schematic Diagram of Phase Offset Measurement Experiment.

Experiments conducted with the RT-AC86U router demonstrate a distinctive pattern in the phase difference between receiving antennas, which is shown in Fig. 4.

Fig. 4. Distribution of phase difference between Rx 2,3,4 with Rx 1.

Specifically, an integrated analysis of data and fitting the distribution using the von Mises distribution in the angular domain reveals a stable phase difference pattern: between antennas 1 and 2, it's around 0.19 radians (concentration

27.63), between 1 and 3, -0.87 radians (concentration 19.61), and between 1 and 4, -1.74 radians (concentration 17.46).

These differences are attributed to inherent antenna properties. Stability is maintained even after system reboots. Consequently, we adjust the CSI for consistent angular offsets and normalize amplitude. The calibrated CSI is represented as: |H (f, t)| · e<sup>jθ</sup>~R~

- e

j∆θ~N~ R(M,N)

H~cali~(M, N) = P~M~ (5)

1

|H~R~(M,N)(f, t)|

M m=1

Where M and N denote the carrier and receiving antenna, respectively, and ∆θ~N~ is the compensating phase for the corresponding antenna. To extract static features related to positioning from CSI data and effectively filter out low-frequency interferences caused by human activities such as walking and breathing, we implemented a preprocessing strategy on CSI data. According to the literature [26], the frequency of human walking is approximately 1.9Hz, and the frequency of breathing ranges between 1 to 1.67Hz. To effectively filter out these low-frequency signals, a high-pass filter approach was adopted. Specifically, we applied a Fast Fourier Transform (FFT) to the CSI data in the time domain, followed by the application of a high-pass filter with a 2Hz cutoff. Subsequently, the data was converted back to its original time domain using an Inverse Fast Fourier Transform (IFFT). This process ensures that features extracted from CSI data are focused on reflecting the spatial information and user position characteristics of the positioning scenario, thereby excluding potential interferences from human dynamic activities. The filtered CSI is represented as: HPF~\>~2Hz(H~cali~(f, t)) = H~s~(f, t) + H~m~(f, t) (6) where HP F~\>~2Hz(H~cali~(f)) denotes the high-pass filtered CSI. Additionally, to further enhance the data's accuracy and stability, a Hampel filter was applied to eliminate potential outliers. The preprocessed data serves as the offline input to the network.

- B. CSI-ResNet

CSI analysis reveals that spatial attributes, stable in contrast to the transient nature of human movement data [23], align closely with the structured nature of visual data. This similarity
