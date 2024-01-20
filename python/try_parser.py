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
from enum import Enum, Flag, auto
import sys
from dataclasses import dataclass


class Layout(Flag):
    none = 0
    layout2013 = auto()
    layout2020 = auto()

    @staticmethod
    def all():
        return Layout.layout2013 | Layout.layout2020

    def to_string(self):
        if Layout.all() in self:
            return "all"
        elif Layout.layout2020 in self:
            return "2020"
        elif Layout.layout2013 in self:
            return "2013"
        else:
            return "none"


class OS(str, Enum):
    LINUX = 'linux'
    MACOS = 'macos'
    WINDOWS = 'windows'


@dataclass
class JobConfig(object):
    name: str
    os: OS = OS.LINUX
    wpt_layout: Layout = Layout.none
    profile: str = "release"
    unit_tests: bool = True
    wpt_tests_to_run: str = ""


def preset(s: str) -> JobConfig | None:
    s = s.lower()

    if s == "linux":
        return JobConfig("Linux", OS.LINUX)
    elif s in ["mac", "macos"]:
        return JobConfig("MacOS", OS.MACOS)
    elif s in ["win", "windows"]:
        return JobConfig("Windows", OS.WINDOWS)
    elif s in ["wpt", "linux-wpt"]:
        return JobConfig("Linux WPT", OS.LINUX, wpt_layout=Layout.all())
    elif s in ["wpt-2013", "linux-wpt-2013"]:
        return JobConfig("Linux WPT legacy-layout", OS.LINUX, wpt_layout=Layout.layout2013)
    elif s in ["wpt-2020", "linux-wpt-2020"]:
        return JobConfig("Linux WPT layout-2020", OS.LINUX, wpt_layout=Layout.layout2020)
    elif s in ["mac-wpt", "wpt-mac"]:
        return JobConfig("MacOS WPT", OS.MACOS, wpt_layout=Layout.all())
    elif s == "mac-wpt-2013":
        return JobConfig("MacOS WPT legacy-layout", OS.MACOS, wpt_layout=Layout.layout2013)
    elif s == "mac-wpt-2020":
        return JobConfig("MacOS WPT layout-2020", OS.MACOS, wpt_layout=Layout.layout2020)
    elif s == "webgpu":
        return JobConfig("WebGPU CTS", OS.LINUX,
                         wpt_layout=Layout.layout2020,  # reftests are mode for new layout
                         wpt_tests_to_run="_webgpu",  # run only webgpu cts
                         profile="production",  # WebGPU works to slow with debug assert
                         unit_tests=False)  # production profile does not work with unit-tests
    else:
        return None


class Encoder(json.JSONEncoder):
    def default(self, o):
        if isinstance(o, (Config, JobConfig)):
            return o.__dict__
        if isinstance(o, Layout):
            return o.to_string()
        return json.JSONEncoder.default(self, o)


class Config(object):
    def __init__(self, s: str | None = None):
        self.fail_fast: bool = False
        self.matrix: list[JobConfig] = list()
        if s is not None:
            self.parse(s)

    def parse(self, s: str):
        s = s.strip()

        if not s:
            s = "linux macos windows"

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


import unittest # noqa
import logging # noqa


class TestParser(unittest.TestCase):
    def test_string(self):
        self.assertEqual(Config("linux").toJSON(),
                         '{"fail_fast": false, "matrix": [\
{"name": "Linux", "os": "linux", "wpt_layout": "none", "profile": "release", \
"unit_tests": true, "wpt_tests_to_run": ""}]}')

    def test_empty(self):
        self.assertEqual(Config("").toJSON(),
                         '{"fail_fast": false, "matrix": [\
{"name": "Linux", "os": "linux", "wpt_layout": "none", "profile": "release", \
"unit_tests": true, "wpt_tests_to_run": ""}, \
{"name": "MacOS", "os": "macos", "wpt_layout": "none", "profile": "release", \
"unit_tests": true, "wpt_tests_to_run": ""}, \
{"name": "Windows", "os": "windows", "wpt_layout": "none", "profile": "release", \
"unit_tests": true, "wpt_tests_to_run": ""}]}')


def run_tests():
    verbosity = 1 if logging.getLogger().level >= logging.WARN else 2
    suite = unittest.TestLoader().loadTestsFromTestCase(TestParser)
    return unittest.TextTestRunner(verbosity=verbosity).run(suite).wasSuccessful()
