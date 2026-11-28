Where $H_{s}(f,t)$ denotes the CSI measurements transmitted by static paths such as ceiling/wall reflections, termed as static CSI component. $H_{m}(f,t)$ and $H_{n}(f,t)$ represent the CSI measurements transmitted via body reflection paths and limb reflection paths, respectively. H(f, t) can also be expressed as:

$$
H(f,t)=\parallel H(f,t)\parallel\cdot e^{i\theta}
$$

(2) Where ∥H(f, t)∥ and $e^{i\theta}$ symbolize the amplitude and phase of the transmitted signal, respectively. Thus, the received CSI can be rewritten as:

$$
\parallel H(f,t)\parallel\cdot e^{j\theta}=\parallel H_{s}(f,t)\parallel\cdot e^{j\theta s}+\parallel H_{m}(f,t)\parallel\cdot e_{j\theta n}^{j\theta m}
$$

(3)

$$+\parallel H_{n}(f,t)\parallel\cdot e$$

However, the phase information in CSI is often affected by factors like carrier frequency offset and sampling frequency offset, leading to significant errors in CSI phase information. Also, the amplitude information in CSI changes with device restarts. Hence, the actual measured CSI is:

$$
\parallel H_{R}(f,t)\parallel\cdot e^{j\theta_{R}}=\alpha(\parallel H_{s}(f,t)\parallel\cdot e_{j\theta e}^{j\theta s}
$$

(4)

$$+\parallel H_{m}(f,t)\parallel\cdot e^{j\theta m}+\parallel H_{n}(f,t)\parallel\cdot e^{j\theta n})e$$

Where $\parallel H_{R}(f,t)\parallel$ is the actual received amplitude, $\theta_{R}$ is the actual received phase, α is the amplitude offset, and $\theta_{e}$ is the intrinsic phase difference between receiving antennas. Therefore, the received CSI requires calibration.

- A. CSI preprocessing As delineated in research [24], [25], frequent rebooting of transceivers impacts oscillator configuration, leading to random phase differences between receiving antennas. For precise intrinsic phase difference acquisition between receiving antennas, we followed the connection scheme depicted in Fig. 3 for transceivers and conducted a series of detailed experimental analyses. After restarting the transceivers 20 times, the phase differences between the receiving antennas were meticulously recorded. By aggregating data across all carriers and time packets, a probability distribution was obtained, as illustrated in Fig. 4.

Fig. 3. Experiment for CSI calibration. (a) Phase Offset Measurement Experiment Setup. (b) Spliter/combiner. (c) Attenuator. (d) Schematic Diagram of Phase Offset Measurement Experiment.

Experiments conducted with the RT-AC86U router demonstrate a distinctive pattern in the phase difference between receiving antennas, which is shown in Fig. 4.

Fig. 4. Distribution of phase difference between Rx 2,3,4 with Rx 1.

Specifically, an integrated analysis of data and fitting the distribution using the von Mises distribution in the angular domain reveals a stable phase difference pattern: between antennas 1 and 2, it's around 0.19 radians (concentration

27.63), between 1 and 3, -0.87 radians (concentration 19.61), and between 1 and 4, -1.74 radians (concentration 17.46).

These differences are attributed to inherent antenna properties. Stability is maintained even after system reboots. Consequently, we adjust the CSI for consistent angular offsets and normalize amplitude. The calibrated CSI is represented as:

∆$\theta_{N}$

H~cali~$(M,N)=\frac{|H_{R(M,N)}(f,t)|\cdot e^{j\theta_{R}}\cdot e^{j}}{M}&#95;{{}^{1}{}^{P_{M}}{}&#95;{m=1}|H_{R(M,N)}(f,t)|}$ (5) Where M and N denote the carrier and receiving antenna, respectively, and ∆$\theta_{N}$ is the compensating phase for the corresponding antenna. To extract static features related to positioning from CSI data and effectively filter out low-frequency interferences caused by human activities such as walking and breathing, we implemented a preprocessing strategy on CSI data. According to the literature [26], the frequency of human walking is approximately 1.9Hz, and the frequency of breathing ranges between 1 to 1.67Hz. To effectively filter out these low-frequency signals, a high-pass filter approach was adopted. Specifically, we applied a Fast Fourier Transform (FFT) to the CSI data in the time domain, followed by the application of a high-pass filter with a 2Hz cutoff. Subsequently, the data was converted back to its original time domain using an Inverse Fast Fourier Transform (IFFT). This process ensures that features extracted from CSI data are focused on reflecting the spatial information and user position characteristics of the positioning scenario, thereby excluding potential interferences from human dynamic activities. The filtered CSI is represented as: HPF${}&#95;{>2Hz}(H$ $(f,t))=H_{s}(f,t)+H_{m}(f,t)$ (6) ~cali~ where $HPF_{>2Hz}(H$ (f)) denotes the high-pass filtered cali CSI. Additionally, to further enhance the data's accuracy and stability, a Hampel filter was applied to eliminate potential outliers. The preprocessed data serves as the offline input to the network.

- B. CSI-ResNet

CSI analysis reveals that spatial attributes, stable in contrast to the transient nature of human movement data [23], align closely with the structured nature of visual data. This similarity
