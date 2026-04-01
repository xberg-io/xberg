# Ubiquitous Smart Eyewear Interactions using Implicit Sensing and Unobtrusive Information Output

Qiushi Zhou
The University of Melbourne
Melbourne, Australia
qiushi.zhou@unimelb.edu.au

Hao-Ping Lee
National Chiao Tung University
Hsinchu, Taiwan
dimension4.cs03@nctu.edu.tw

Joshua Newn
The University of Melbourne
Melbourne, Australia
joshua.newn@unimelb.edu.au

Chaofan Wang
The University of Melbourne
Melbourne, Australia
chaofanw@student.unimelb.edu.au

Benjamin Tag
The University of Melbourne
Melbourne, Australia
benjamin.tag@unimelb.edu.au

Eduardo Velloso
The University of Melbourne
Melbourne, Australia
eduardo.velloso@unimelb.edu.au

**ABSTRACT**

Premature technology, privacy, intrusiveness, power con-
sumption, and user habits are all factors potentially con-
tributing to the lack of social acceptance of smart glasses.
After investigating the recent development of commercial
smart eyewear and its related research, we propose a design
space for ubiquitous smart eyewear interactions while max-
imising interactivity with minimal obtrusiveness. We focus
on implicit and explicit interactions enabled by the combina-
tion of miniature sensor technology, low-resolution display
and simplistic interaction modalities. Additionally, we are
presenting example applications outlining future develop-
ment directions. Finally, we aim at raising the awareness of
designing for ubiquitous eyewear with implicit sensing and
unobtrusive information output abilities.

**CCS CONCEPTS**
- **Human-centered computing** → *User interface toolk-
its; Ubiquitous computing; Ubiquitous and mobile comput-
ing systems and tools; Mobile devices.*

**KEYWORDS**

eyewear, wearable, sensor, context awareness

ACM Reference Format:
Qiushi Zhou, Joshua Newn, Benjamin Tag, Hao-Ping Lee, Chao-
fan Wang, and Eduardo Velloso. 2019. Ubiquitous Smart Eyewear
<!-- image -->
Interactions using Implicit Sensing and Unobtrusive Information
Output. In Adjunct Proceedings of the 2019 ACM International Joint
Conference on Pervasive and Ubiquitous Computing and the 2019 In-
ternational Symposium on Wearable Computers (UbiComp/ISWC '19
Adjunct), September 9-13, 2019, London, United Kingdom. ACM, New
York, NY, USA, 6 pages. https://doi.org/10.1145/3341162.3348392

**1 INTRODUCTION**

Humans interact with the world by perceiving it through sen-
sory input, identifying action affordance and then producing
motor output [7]. As most sensory input are received by the
organs on human face, it makes the face especially interest-
ing for collecting physiological data as well as for delivering
information for Human-Computer Interaction. Glassses are
among the most common devices that we wear on our faces
daily. They have the natural advantage as potential interac-
tive devices by always being available within close proximity
with the main sensory organs, for physiological information
input and discrete yet unobtrusive information output.
Since the 13th century, spectacles (prescription glasses)
have been used primarily to alleviate visual impairments,
and later developed as a fashion accessory [12]. Glasses with
computational functionality and sensing abilities, neverthe-
less, have not yet reached that level of acceptance. Smart
glasses and Head-Mounted-Device (HMD) have been added
with layers of high-tech functions by ambitious developers
while only resulting in low social acceptance due to their
obtrusive appearance, form factor and other issues.
While smart eyewear with high-resolution displays com-
bined with sophisticated input technologies present their
own set of challenges, sole sensing devices with no displays
remain limited in their usability beyond specific research
applications. The rich potential of outputting information
is hence wasted. An example are the JINS MEME glasses
that come equipped with sensors to track physiological sig-
nals but rely on an external device, such as a computer or a
smartphone to process the collected data and to display the
acquired insights. While these devices are characterised by

<!-- image -->

**2 RELATED WORK**

As smart wearable devices gain traction from consumers as
ubiquitous computers, a range of smart eyewear devices has
emerged in the market. Head-Mounted Displays (HMD) such
as Microsoft HoloLens¹ and Magic Leap² are equipped with
state-of-the-art holographic projection technology to enable
augmented reality applications and interactions. Other de-
vices provide similar functions at a smaller scale, such as
Google Glass³, Focals⁴ and Vuzix Blade5. These glasses utilise
projection technology to achieve high-resolution informa-
tion display with smaller form factor. However, the HMDs
and the smart glasses are often costly to purchase and their
obtrusive design makes them unsuitable for everyday use,
especially while interacting face-to-face with others.
Other commercial smart eyewear abandoned advanced
display technologies for a more compact form factor to suit
everyday use scenarios. Vue supports essential functions
that require connectivity with a smartphone. It provides
functions such as calling, navigation and a music player im-
plemented with bone conduction for audio output and an
integrated touchpad for control. It is also equipped with Iner-
tial Measurement Unit (IMU) and infrared proximity sensors
for activity tracking. Other eyewear devices are explicitly
designed for sensing purposes. JINS MEME is equipped
with Electrooculography (EOG) and IMU sensors for physio-
logical data logging in everyday settings. Pupil Invisible is
embedded with miniature infrared eye trackers to provide
everyday gaze tracking. Due to the lack of output channels
for these devices, their appeal to the general public is lim-
ited. Privacy is another concern regarding the use of Pupil

¹https://www.microsoft.com/en-us/hololens
²https://www.magicleap.com/
³https://www.google.com/glass/start/
⁴https://www.bynorth.com/
⁵https://www.vuzix.com/products/blade-smart-glasses
⁶https://www.enjoyvue.com/
⁷https://jins-meme.com/en/
⁸https://pupil-labs.com/news/2019-01-09/pupil-invisible-beta-launch.html

Invisible because of its discrete environmental data collec-
tion ability. Other eyewear such as SKUGGA and CTRL
Eyewear¹⁰ provide automatic tinting control using ambient
light sensors without any interaction functionality.
Equipping eyewear with miniature sensing abilities pro-
vides opportunities for researchers to explore and expand the
application realm of these devices. Zhou et al. [15] presented
the feasibility of measuring mental workload with infrared
thermal sensors in the lab setting. Tag et al. [13] assessed
fatigue levels by capturing participants' (EOG) signals in-
the-wild. Uema and Inoue [14] used EOG data to measure
users' concentration levels in different contexts. Ishimaru
et al. [4] captured eye blink data using infrared sensors to
predict users' high-level on-going activities and Dementyev
and Holz [3] used IR information to log blink frequencies
and alleviate dry eyes by triggering blinks through stimuli.
Other than for data collection purposes, researchers also
exploited sensors to combine with other miniature technolo-
gies for alternative output modalities. Rantala et al. [10] pro-
posed gaze gestures combined with haptic feedback to enable
reliable input on such devices. Costanza et al. [1] utilized
peripheral vision to deliver less obtrusive visual notifications
using such devices. Olwal and Kress [9] used low-power
LED on the inner side of the frame for navigation guidance
and projected pre-recorded computer-generated holographic
(CGH) icons for limited in-lens display.
The often bulky form factor induced by excessive func-
tionality in high-end smart glasses and the scarcity of output
channels in eyewear for sensing purposes both contribute to
the lack of social acceptance of those devices as ubiquitous
eyewear for everyday use. We propose an intermediate de-
sign space on the functionality-simplicity spectrum for an
everyday smart eyewear design.

**3 DESIGN SPACE FORMULATION**

Glasses and wristwatches are two of the most common
functional gadgets found on our bodies in our everyday
life [5, 8]. Despite sharing the early inceptions, the two
devices followed distinct digitisation processes. Whereas
analogue watches face steadily decreasing user numbers,
smartwatches are catching up [2]. During the evolution of
analogue watches to becoming smartwatches, hybrid sys-
tems were developed. The development presented with de-
vices that displayed limited digital information, with low-
resolution digital watches utilizing numerical interfaces, and,
more recently, fitness wristbands with similar displays. These
low-resolution devices are still popular despite the advent of
smartwatches because of their simplistic functionality, low
cost and often specific use scenarios.

⁹http://skuggaeyewear.com/
¹⁰http://www.e-tintproducts.com/ctrl-eyewear/

<!-- image -->
<!-- image -->

In contrast to the gradual evolution of smartwatches, the
evolution of smart eyewear has included a discontinuity-it
is not as easy to find devices which sit between traditional
prescription glasses and smart glasses. Smart glasses, such
as Google Glass and Focals, share similar functionality with
smartwatches as secondary and ubiquitous digital displays
embedded on devices which are already habitually worn
by users. Even though there is a larger population wearing
glasses than that wearing watches, smart glasses are rarely
seen in everyday life compared to smartwatches [5, 8]. Po-
tential reasons for lower social acceptance include obtrusive
form factor, distraction and privacy concerns and conflict
with existing prescription glasses.
Figure 1 illustrates the motivation behind our proposed
design space. Inspired by the evolution of smartwatches, we
propose to bridge the gap between everyday glasses and
smart glasses with an intermediate type of eyewear. We
formulate a design space for eyewear with unobtrusive form
factor, low-resolution display, simplistic functionality and
context-awareness through environmental and physiological
sensing. These devices should be easily incorporated with
prescription frame glasses without structural modification.
Table 1 summarises our design space analysis covering
different input and output modalities in implicit and explicit
forms. We focus on implicit input and low-resolution output
modalities to explore interaction affordance.

Input Modalities

In this work, we use Schmidt's definition of *implicit input* as
"an action, performed by the user that is not primarily aimed
to interact with a computerized system but which such a
system understands as input." We define *explicit input* as

<!-- image -->

|  | Implicit | Explicit |
| --- | --- | --- |
| Input Modality | Gaze direction; Stress; Workload; Fatigue; Head Movement; Facial Expression | Touch; Smooth Pursuit |
| Output Modality | Automatic tinting; Background data logging; Peripheral notification | Peripheral display; LED; See-through OLED; Symbolic holograph; Audio; Haptic |

Table 1: Design Space

users' deliberate manipulation of the device for the purposes
of interacting with it [11].

*Implicit Input*. The numerous miniaturised environmental
and physiological sensors embedded in existing wearable de-
vices enable potential input modalities with implicit context
detection and data collection without users' awareness. Gaze
direction can be detected by training algorithms to recognise
patterns in EOG signal from users' eyes. Electrodermal activ-
ity (EDA), facial temperature and blink rate enable detection
of users' cognitive state variations including stress, workload
and alertness. Miniature EDA sensors and infrared temper-
ature sensors can be unobtrusively embedded in eyewear
frames and blink rate can be captured by EOG sensors. Nat-
ural movement from users' head can be detected by the IMU
sensors and used for activity logging and motion-related
interaction. Facial expression can be detected by training
algorithms to recognise facial skin deformation with photo
reflective sensors [6]. Ambient light sensors can inform the
device when the lighting condition is uncomfortable.

*Explicit Input*. In the context of ubiquitous smart eyewear
interaction, we explore simplistic explicit input modalities
which do not require complex hardware support. Instead of
advanced gesture recognition as implemented in HoloLens,
simple directional touch gestures can be utilised by embed-
ding miniature capacitive touchpads on the outside of the
temples. EOG signals can be used to utilize eye movements
for smooth pursuit interactions and to enable calibration-free
gaze input.

Output Modalities

In this work, we define *explicit output* as the presentation of
information which leads to users' immediate awareness. We
define *implicit output* as notifications or background data
logging which do not require users' immediate attention or
continuous engagement.

*Implicit Output*. As discussed above, physiological sensors
enable passive background data logging. This can be used to
monitor vital signs and and other health or fitness-related
indicators such as the number of steps walked, heart rate,
blood oxygen levels, and distance travelled within certain
time periods¹¹¹². Costanza et al. have used LED lights in the
peripheral vision field, which can be attached to the inside
of eyewear temples to enable implicit notifications without
occupying users' attention [1]. Subtle self-tinting can provide
users with a comfortable vision in combination with ambient
light sensors without their notice.

*Explicit Output*. In the context of this work, we focus on the
simplistic presentation of notification and other explicit infor-
mation which induce users' immediate awareness but do not
require their continuous engagement. LEDs can be embedded
and activated at different locations on the frame to present di-
rectional information. Transparent OLED screens with small
form factor, such as SparkFun Transparent OLED¹³, provide
a simplistic graphical display for necessary information such
as time, temperature, and notification summary that is avail-
able with limited computing power and power consumption.
Symbolic holograph projections are a visual alternative to
graphical displays which present symbolic information from
a predefined set, instead of continuously generating new
patterns. Acoustic, haptic and bone conduction output can
present notifications or as secondary feedback modalities to
compensate for other primary output modalities [10].

**4 APPLICATIONS**

With our enumeration of potential interactions afforded, we
illustrate three applications as examples of implementing
our design space. We focus on practical everyday scenarios
to explore different combinations of contextual sensor input
and unobtrusive output.

Context-Aware Information

Combining head movement detection by IMU sensors with
physical contextual information collected by environmental
sensors, the device can infer users' underlying intentions
and provide simple feedback. For example, time, weather and
agenda icons can be displayed as a holographic projection
or transparent OLED content when users look up at the sky.

Implicit Assistance

Implicit assistance contributes to better accessibility in sce-
narios where users could not reach other devices, such as

¹¹https://www.apple.com/au/apple-watch-series-4
¹²https://buy.garmin.com/en-AU/AU/p/605739
¹³https://core-electronics.com.au/sparkfun-transparent-oled-hud-
breakout-qwiic.html

<!-- image -->

|  | Implicit Input | Explicit Output |
| --- | --- | --- |
| Implicit Output | Automatic tinting under sunlight; Unobtrusive message notification during task [1]; Background lifelogging (heart rate, steps, etc.); Facial expression detection; Stress and fatigue detection. | Lifelogging with camera. |
| Explicit Output | Drowsiness alert; Attention alert; Emotion regulation; Stress regulation. | Notification responding; Face-touching input; Smooth pursuit. |

Table 2: Applications

phones and computers, while being occupied with demand-
ing tasks. For example, symbolic left/right arrows can be
displayed during running or cycling to support navigation.
Symbolic notifications can be displayed to alert users if nega-
tive physiological features are detected such as fatigue, stress,
and drowsiness. For example, this feature could be useful in
detecting fatigue driving.

Cross-Device Interaction

As everyday wearable devices, smart eyewear has the ad-
vantage of ubiquitous availability. While smartphones can
display sophisticated semantic and graphical information,
users can spend an unnecessarily long time to find the in-
formation intended through different software due to the
overpowering interactivity. This process can be further sim-
plified by combining mobile interaction with smart eyewear
while utilizing its sensing abilities. For example, users can
receive a notification of an incoming email as an icon dis-
played on the glasses. Instead of unlocking the phone and
navigating to the email interface, the new email can auto-
matically present itself after the phone is unlocked when the
glasses detect that the user is facing the phone while there
is an incoming email icon displayed.

**5 CONCLUSION**

In this paper, we first identified the problem of the low accep-
tance of smart glasses by society and discussed its potential
reasons. We summarised previous research works exploring
alternative input and output modalities with smart eyewear.
We also investigated state-of-the-art commercial technology-
embedded eyewear with novel sensing abilities. Inspired by
the evolution of smartwatches, we formulated a design space
for eyewear with intermediate interaction affordance which
utilises miniature sensing, low-resolution displays and other
assistive interaction modalities. Finally, we proposed exam-
ple applications utilising different combinations of implicit
and explicit interaction modalities to call attention to future
works towards ubiquitous smart eyewear interaction.

**REFERENCES**

[1] Enrico Costanza, Samuel A. Inverso, Elan Pavlov, Rebecca Allen, and
Pattie Maes. 2006. Eye-q: Eyeglass Peripheral Display for Subtle In-
timate Notifications. In Proceedings of the 8th Conference on Human-
computer Interaction with Mobile Devices and Services (MobileHCI '06).
ACM, New York, NY, USA, 211-218. https://doi.org/10.1145/1152215.
1152261 event-place: Helsinki, Finland.

[2] Tony Danova. 2015. THE SMARTWATCH REPORT: The market for
luxury wristwatches, retail distribution, and Apple WatchâĂŹs big
opportunity.
https://www.businessinsider.com/the-smartwatch-
report-the-market-for-luxury-wristwatches-retail-distribution-and-
apple-watchs-big-opportunity-2015-2

[3] Artem Dementyev and Christian Holz. 2017. DualBlink: A Wearable
Device to Continuously Detect, Track, and Actuate Blinking For Alle-
viating Dry Eyes and Computer Vision Syndrome. Proceedings of the
ACM on Interactive, Mobile, Wearable and Ubiquitous Technologies 1, 1
(March 2017), 1-19. https://doi.org/10.1145/3053330
[4] Shoya Ishimaru, Kai Kunze, Koichi Kise, Jens Weppner, Andreas Den-
gel, Paul Lukowicz, and Andreas Bulling. 2014. In the Blink of an
Eye: Combining Head Motion and Eye Blink Frequency for Activity
Recognition with Google Glass. In Proceedings of the 5th Augmented
Human International Conference (AH '14). ACM, New York, NY, USA,
15:1-15:4. https://doi.org/10.1145/2582051.2582066 event-place: Kobe,
Japan.
[5] Natasha Lomas. 2014. Global wearables market to grow
17% in 2017, 310M devices sold, $30.5BN revenue: Gartner.
http://social.techcrunch.com/2017/08/24/global-wearables-market-
to-grow-17-in-2017-310m-devices-sold-30-5bn-revenue-gartner/
[6] Katsutoshi Masai, Yuta Sugiura, Masa Ogata, Kai Kunze, Masahiko
Inami, and Maki Sugimoto. 2016. Facial Expression Recognition in
Daily Life by Embedded Photo Reflective Sensors on Smart Eyewear.
In Proceedings of the 21st International Conference on Intelligent User
Interfaces - IUI '16. ACM Press, Sonoma, California, USA, 317-326.
https://doi.org/10.1145/2856767.2856770
[7] Bence Nanay. 2013. Between perception and action. Oxford University
Press.
[8] Statistics Netherlands. 2013. More than 6 in 10 people wear glasses or
contact lenses. https://www.cbs.nl/en-gb/news/2013/38/more-than-
6-in-10-people-wear-glasses-or-contact-lenses
[9] Alex Olwal and Bernard Kress. 2018. 1D eyewear: peripheral, hidden
LEDs and near-eye holographic displays for unobtrusive augmentation.
In Proceedings of the 2018 ACM International Symposium on Wearable
Computers - ISWC '18. ACM Press, Singapore, Singapore, 184-187.
https://doi.org/10.1145/3267242.3267288
[10] Jussi Rantala, Jari Kangas, Poika Isokoski, Deepak Akkil, Oleg Åăpakov,
and Roope Raisamo. 2015. Haptic Feedback of Gaze Gestures with
Glasses: Localization Accuracy and Effectiveness. In Adjunct Proceed-
ings of the 2015 ACM International Joint Conference on Pervasive and
Ubiquitous Computing and Proceedings of the 2015 ACM International
Symposium on Wearable Computers (UbiComp/ISWC'15 Adjunct). ACM,
New York, NY, USA, 855-862. https://doi.org/10.1145/2800835.2804334
event-place: Osaka, Japan.

[11] Albrecht Schmidt. 2005. Interactive context-aware systems interacting
with ambient intelligence. Ambient intelligence 159 (2005).
[12] E Temple Smith. 1928. THE HISTORY OF SPECTACLES. Medical
Journal of Australia 2, 19 (1928), 578-587. https://doi.org/10.5694/j.
1326-5377.1928.tb13682.x
[13] Benjamin Tag, Andrew W. Vargo, Aman Gupta, George Chernyshov,
Kai Kunze, and Tilman Dingler. 2019. Continuous Alertness Assess-
ments: Using EOG Glasses to Unobtrusively Monitor Fatigue Levels
In-The-Wild. In Proceedings of the 2019 CHI Conference on Human Fac-
tors in Computing Systems - CHI '19. ACM Press, Glasgow, Scotland
Uk, 1-12. https://doi.org/10.1145/3290605.3300694

[14] Yuji Uema and Kazutaka Inoue. 2017. JINS MEME algorithm for esti-
mation and tracking of concentration of users. In Proceedings of the
2017 ACM International Joint Conference on Pervasive and Ubiquitous
Computing and Proceedings of the 2017 ACM International Symposium
on Wearable Computers. ACM, 297-300.
[15] Qiushi Zhou, Joshua Newn, Namrata Srivastava, Tilman Dingler, Jorge
Goncalves, and Eduardo Velloso. 2019. Cognitive Aid: Task Assistance
Based On Mental Workload Estimation. In Extended Abstracts of the
2019 CHI Conference on Human Factors in Computing Systems (CHI
EA '19). ACM, New York, NY, USA, LBW2315:1-LBW2315:6. https:
//doi.org/10.1145/3290607.3313010 event-place: Glasgow, Scotland Uk.
