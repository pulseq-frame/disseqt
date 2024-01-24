# Disseqt - dissect MRI sequences

This crate provides a minimalistic interface around reading MRI sequences (currently only pulseq is supported via [pulseq-rs](https://github.com/pulseq-frame/pulseq-rs)).
It can be used to plot, simulate, convert MRI sequences. The API purposefully does not expose any internal details about the sequence, which means any implementations based on it should be forward compatible to any new file format supported by disseqt.
