# Improved Contextual Recognition in Automatic Speech Recognition Systems

**by Semantic Lattice Rescoring**

1 2 4 5,6†

## Ankitha Sudarshan , Vinay Samuel , Parth Patwa<sup>3</sup>, Ibtihel Amara , Aman Chadha

1 2 3

## Purdue University Carnegie Mellon University University of California Los Angeles

4 5 6

## McGill University Stanford University Amazon AI

1 5,6

sudarsh0@purdue.edu hi@aman.ai

## Abstract

Automatic Speech Recognition (ASR) has witnessed a profound research interest. Recent breakthroughs have given ASR systems different prospects such as faithfully transcribing spoken language, which is a pivotal advancement in building conversational agents. However, there is still an imminent challenge of accurately discerning context-dependent words and phrases. In this work, we propose a novel approach for enhancing contextual recognition within ASR systems via semantic lattice processing leveraging the power of deep learning models in accurately delivering spot-on transcriptions across a wide variety of vocabularies and speaking styles. Our solution consists of using Gaussian Mixture Models and Hidden Markov Models (GMM-HMM) along with Deep Neural Networks (DNN) models integrating both language and acoustic modeling for better accuracy. We infused our network with the use of a transformerbased model to properly rescore the word lattice achieving remarkable capabilities with a palpable reduction in Word Error Rate (WER). We demonstrate the effectiveness of our proposed framework on the LibriSpeech dataset with empirical analyses.

**Index terms** - speech recognition, lattice re-scoring, contextual speech recognition, word lattices

## 1\. Introduction

ness and adaptability of ASR systems, aiming to minimize errors and misunderstandings that can arise from speech variability. Furthermore, the advent of global digitalization has led to an unprecedented intermingling of cultures and languages, introducing additional layers of complexity to speech recognition. The diversity in accents and dialects, even within the same language, poses a considerable challenge for ASR systems. This diversity necessitates the development of sophisticated models that can adapt to the nuances of spoken language across different regions and communities. Additionally, the rapid expansion of smart home devices relies heavily on effective speech recognition capabilities. As these technologies become more embedded in our daily lives, the ability of ASR systems to accurately interpret commands and queries becomes crucial for user experience. Incorrect recognition can lead to frustration, errors, and in some contexts, serious consequences. The interaction between humans and machines through voice commands is predicated on the ability of the system to understand the intent and context of the spoken words. This interaction becomes even more challenging in noisy environments or when the speech signal is distorted, highlighting the need for robust ASR systems that can perform well under less-than-ideal conditions.

Recognizing spoken language accurately and efficiently is a complex task due to variability in the source of speech such as pronunciation, dialects, vocabulary, accents, articulation, etc. This variability presents significant challenges in the development and fine-tuning of Automatic Speech Recognition (ASR) systems. Different speakers can produce vastly different speech signals even when uttering the same word or phrase, complicating the task of accurate speech recognition. Factors such as background noise, speaker distance from the microphone, and the acoustic environment further exacerbate these challenges,

necessitating sophisticated solutions to ensure ASR systems can perform reliably under a wide range of conditions. The importance of addressing these variabilities cannot be overstated, as they significantly affect the accuracy and reliability of ASR systems. These systems are increasingly becoming integral in various applications, from virtual assistants to real-time translation services, thereby amplifying the need for improved precision. Moreover, the rising demand for hands-free operations and accessibility features for individuals with disabilities further underscores the urgency of refining ASR technology. To this end, researchers and developers are continuously exploring innovative approaches to enhance the robust-

†Work does not relate to position at Amazon. Technological advancements in artificial intelligence have opened new frontiers in enhancing ASR accuracy. However, the pace at which spoken language evolves, with new slang, terminologies, and phrases emerging constantly, presents an ongoing challenge. Keeping ASR systems updated and relevant requires not just technological innovation but also a deep understanding of linguistic dynamics and cultural nuances. Semantic interpretation of a speech is crucial in ASR systems. Let us take the following example. "I am going to a bank to deposit a check". Without context, the word bank could refer to either a financial institution or either the edge of a river illustrating the critical need for ASR systems to contextualize language effectively. This ambiguity underscores the complexity of human language and highlights the limitations of current ASR systems in understanding context and disambiguating meaning. By addressing this gap, we can significantly improve the functionality and applicability of ASR technology across a broad spectrum of use-cases. To bridge this contextual gap in ASR systems, semantic lattice processing is a key component in contributing to better recognition of situational context conditions. This technique utilizes a lattice structure to represent the relationships between words and phrases in a sentence. It is created by analyzing the audio input and identifying possible word and phrase combinations and their associated prob-
