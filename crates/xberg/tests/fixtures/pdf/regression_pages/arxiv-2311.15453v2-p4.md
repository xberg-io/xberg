AP

**ATLAS BraTS-T1 BraTS-T2**

25 0.03 ± 0.00 0.38 ± 0.01 0.23 ± 0.01 50 0.20 ± 0.02

**0.44** ± 0.02 0.45 ± 0.01

t

75

**0.29** ± 0.03 0.34 ± 0.03 0.59 ± 0.03 100 0.25 ± 0.03 0.28 ± 0.02

**0.70** ± 0.02

**single-step**

10 0.27 ± 0.02 0.32 ± 0.02 0.73 ± 0.02 20 0.29 ± 0.02 0.34 ± 0.02 0.74 ± 0.02

size

25 0.29 ± 0.02 0.34 ± 0.02 0.75 ± 0.02 33 0.29 ± 0.02 0.35 ± 0.02 0.75 ± 0.01

**multi-step**

step

50 0.30 ± 0.03 0.35 ± 0.02 0.75 ± 0.02

**Table 1** . Upper section: AP for single-step restorations

(AS = ∥x−P(x, t)∥) as a function of the t used to condition the model. Lower section: AP for multi-step restorations, as a function of step size. Mean and std. dev. on 4 seeds.

**Fig. 3** . Restoration results on test images drawn from the AT- LAS [17], BraTS-T1, and BraTS-T2 [18], after step size = 25, along with anomaly scores AS in grayscale compared with the ground truth outline in red.

When using single-step restorations we found that AP peaks at different t values for different anomalies and modal- **Fig. 2** . AP profiles as a function of t. Blue line shows singleities (Fig. 2). We hypothesize that these profiles are associstep restoration. Orange line shows the cumulative AS as deated with how the anomalies manifest in each image modality, fined in Section 2 up-to t stage (step size = 25). Single seed.

Method VAE [2] r-VAE [5]

**IR**

f-AnoGAN [3] FAE [19]

**FM**

RD [20] AMCons [21]

**AB**

PII [12] DAE [9]

**S-S**

MOOD22 [13] DISYRE (Ours)

with high-frequency anomalies being restored at lower t values and more structural corruptions restored as t approaches

**ATLAS BraTS-T1 BraTS-T2**

T. We aim to be sensitive to all classes of anomaly, from

AP ⌈Dice⌉ AP ⌈Dice⌉ AP ⌈Dice⌉

structural to high frequency, which is why we integrate mul-

0.11 0.20 0.13 0.19 0.28 0.33

tiple steps into the proposed AS. Despite improving results

0.09 0.17 0.13 0.19 0.36 0.40

0.02 0.06 0.06 0.12 0.15 0.21

in BraTS-T2 and ATLAS, multi-step AS does not improve

0.08 0.18

**0.42 0.45**

0.51 0.52

single-step AS in BraTS-T1, potentially due to the nature of

0.11 0.22 0.36 0.42 0.47 0.50

how gliomas manifest in T1-weighted Brain MRI. In future

0.01 0.03 0.05 0.12 0.35 0.40

work we will explore alternative approaches to achieve better

0.03 0.07 0.13 0.22 0.13 0.22

coverage of the anomaly spectrum.

0.05 0.13 0.13 0.20 0.47 0.49

As seen in Table 1, DISYRE is robust to the choice of the

0.10 0.21 0.24 0.31 0.47 0.48

0.19 0.27 0.22 0.30 0.69 0.67

stepsize hyper-parameter. We opted for stepsize = 25 in

DISYRE f.w. (Ours)

**0.29 0.37**

0.34 0.41

**0.75 0.70**

our final experiments as it provides a good balance between coverage of the anomaly schedule and inference speed.

**Table 2** . Localization results of the image-reconstruction

In this work we propose DISYRE, a new strategy that ( **IR** ), feature-modeling ( **FM** ), attention-based ( **AB** ), and selfadapts diffusion models to UAD. With the motivation of supervised ( **S-S** ) methods. Best scores are bold. learning a score function ∇~x~log p(x) which generalizes to naturally occurring medical anomalies, we replace the standard Gaussian noise corruption with gradual, synthetic

4. **DISCUSSION & CONCLUSION**

anomalies. DISYRE opens a new route to leverage diffusion- Results in Table 2 highlight the effectiveness of our strategy:

- like models in medical image analysis;
- we show that forward DISYRE achieves an AP of 0.75 in BraTS-T2 dataset, an in- processes can be designed so they become relevant for downcrease of 0.24 from the second best method. DISYRE also stream tasks. In DISYRE we leverage synthetic anomalies improves the state of the art of the ATLAS task to 0.29, up for UAD and substantially outperform other UAD baseline from 0.19 while achieving competitive results in BraTS-T1. methods in two out of three datasets.
