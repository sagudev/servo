#!/usr/bin/env python

# Copyright 2023 The Servo Project Developers. See the COPYRIGHT
# file at the top-level directory of this distribution.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

import json
from enum import Enum
import sys


class Layout(str, Enum):
    none = 'none'
    layout2013 = '2013'
    layout2020 = '2020'
    all = 'all'


class OS(str, Enum):
    linux = 'linux'
    mac = 'mac'
    win = 'windows'


class JobConfig(object):
    """
    Represents one config tuple

    name[os=_, layout=_, ...]
    """

    def __init__(self, name: str, os: OS = OS.linux, layout: Layout = Layout.none,
                 profile: str = "release", unit_test: bool = True, wpt: str = ""):
        self.os: OS = os
        self.name: str = name
        self.wpt_layout: Layout = layout
        self.profile: str = profile
        self.unit_tests: bool = unit_test
        self.wpt_tests_to_run: str = wpt


def preset(s: str) -> JobConfig | None:
    s = s.lower()

    if s == "linux":
        return JobConfig("Linux", OS.linux)
    elif s == "mac":
        return JobConfig("MacOS", OS.mac)
    elif s in ["win", "windows"]:
        return JobConfig("Windows", OS.win)
    elif s in ["wpt", "linux-wpt"]:
        return JobConfig("Linux WPT", OS.linux, layout=Layout.all)
    elif s in ["wpt-2013", "linux-wpt-2013"]:
        return JobConfig("Linux WPT legacy-layout", OS.linux, layout=Layout.layout2013)
    elif s in ["wpt-2020", "linux-wpt-2020"]:
        return JobConfig("Linux WPT layout-2020", OS.linux, layout=Layout.layout2020)
    elif s in ["mac-wpt", "wpt-mac"]:
        return JobConfig("MacOS WPT", OS.mac, layout=Layout.all)
    elif s == "mac-wpt-2013":
        return JobConfig("MacOS WPT legacy-layout", OS.mac, layout=Layout.layout2013)
    elif s == "mac-wpt-2020":
        return JobConfig("MacOS WPT layout-2020", OS.mac, layout=Layout.layout2020)
    elif s == "webgpu":
        return JobConfig("WebGPU CTS", OS.linux, layout=Layout.layout2020, wpt="_webgpu",
                         profile="production",  # WebGPU works to slow with debug assert
                         unit_test=False)  # production profile does not work with unit-tests
    else:
        return None


class Encoder(json.JSONEncoder):
    def default(self, o):
        if isinstance(o, (Config, JobConfig)):
            return o.__dict__
        return json.JSONEncoder.default(self, o)


class Config(object):
    def __init__(self, s: str | None = None):
        self.fail_fast: bool = False
        self.matrix: list[JobConfig] = list()
        if s:
            self.parse(s)

    def parse(self, s: str):
        s = s.strip()

        if not s:
            s = "full"

        for m in s.split(" "):
            p = preset(m)
            if p is None:
                print(f"Ignoring wrong preset {m}")
            else:
                self.matrix.append(p)

    def toJSON(self) -> str:
        return json.dumps(self, cls=Encoder)


def main():
    conf = Config(" ".join(sys.argv[1:]))
    print(conf.toJSON())


if __name__ == "__main__":
    main()
