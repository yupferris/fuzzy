# fuzzy

Virtual Boy serial testing stuff, currently WIP.

# link port

The link port on the Virtual Boy is nearly identical to the one on the Gameboy (which is unsurprising, given it was designed by the same hardware designer). The two main differences are that the Virtual Boy's link port contains two extra pins that are used to synchronize the display hardware between the two systems, and the master clock has a faster rate (50khz in the Virtual Boy vs 8192hz in the Gameboy).

[![Virtual Boy link port](http://www.planetvb.com/modules/dokuwiki/lib/exe/fetch.php?cache=cache&media=link_port.png)](http://www.planetvb.com/modules/dokuwiki/doku.php?id=link_port)

Note the two bidirectional `Control` and `Clock` lines. These lines are each connected to open collector outputs tied to inputs and pull-up resistors. This way, either VB can pull either of these lines low, possibly at the same time without the risk of damaging any of the hardware on the other end. If neither unit pulls these lines low, the pull-up resistors pull the lines high.

The link port protocol is very simple and transmits a single byte at a time in full duplex (each unit both sends and receives at the same time). One unit is designated `master`, the other `slave`. For each byte, the `master` unit pulls the `Control` line low, indicating that a transfer will take place. The `master` will then pulse the `Clock` line low 8 times. When a Virtual Boy unit is acting as the `master`, these `Clock` pulses have a period of 20μs. The 8 data bits being transferred by each unit (MSB first) are latched on the rising edges of these pulses. Finally, the `master` unit will release the `Control` line, and the transfer is complete.

# wiring setup

The teensy-based setup used here only connects 5 lines to the port, including ground. All of the signal lines are connected to port B on the teensy.

VB link port pin | Teensy contact | Purpose | Wire color in my setup
--- | --- | --- | ---
7 | GND | Ground | green
1 | B0 | Control | black
3 | B1 | Clock | white
4 | B2 | Data in (to VB, data out from the teensy's perspective) | orange
8 | B3 | Data out (from VB, data in from the teensy's perspective | blue

# license

Unless otherwise stated in specific files/directories, this code is licensed under the MIT license (see LICENSE).
