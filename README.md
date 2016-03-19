Antikörper
==========

[![Build Status](https://travis-ci.org/anti-koerper/antikoerper.svg?branch=master)](https://travis-ci.org/anti-koerper/antikoerper)
[![Clippy Linting Result](https://clippy.bashy.io/github/anti-koerper/antikoerper/master/badge.svg)](https://clippy.bashy.io/github/anti-koerper/antikoerper/master/log)

Using [homu](https://homu.io) we assure that we have an evergreen master!


Antikörper is meant to be a lightweight data aggregation and visualization tool.

It's basic idea is to aggregate data from your PC. You can then derive
projections from this data, use it any way you find useful.

Possible applications:

- Battery Usage
- Analyze your own PC usage (Which programs are focused when your PC is not
idle?)
- Time spent listening to Music
- Anything you can think of!


Naming
------

The name Antikörper is german for antibody. The idea is that it is there, in the
background, easily forgotten, but nonetheless busy and useful.

Config File
-----------

The config file is a simple toml file that is read at program start. It allows
you to specify which aspects of your Computer should be monitored and in which
intervals.

A sample config with all options used:

```toml
[general]
shell = "/usr/bin/bash"

[[items]]
key = "os.battery"
interval = 60
env = { actually = "not used here" }
command = "acpi"

[[items]]
key = "os.usage"
interval = 1
shell = "cat /proc/loadavg | cut -d' ' -f1"

[[items]]
key = "backlight.brightness"
interval = 10
file = "/sys/class/backlight/intel_backlight/actual_brightness"
```

### Section `general`

- `shell`, the default shell is `/usr/bin/sh`. If you want to use another one,
  specify it here.

### Section `items`

`items` is an array, we (ab)use the toml syntax to make nice looking config
files.

Each item needs to have these keys:
- `key`, the key of the value that the programm will return.
- `interval`, the interval between two 'runs'
- `file` OR `shell` OR `command`, only one can be specified.

It can optionally take these:
- `env`, a map of key = values, to set environment variables

# LICENSE

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.


--------


__Copyright (C) 2016 Marcel Müller (neikos at neikos.email)__
