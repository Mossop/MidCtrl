# MidiCtrl

A currently experimental tool to wire up a MIDI controller to Lightroom.

Yes, many of these already exist, none really did exactly what I wanted though and I quickly found
them frustrating to use.

Plus I've been looking for a good excuse to get back into Rust.

This is split into two parts, a Lightroom plugin for getting the current Lightroom state and making changes and a Rust binary for talking to the plugin and any MIDI controllers.
