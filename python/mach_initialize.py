# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at http://mozilla.org/MPL/2.0/.

# based off https://searchfox.org/mozilla-central/source/build/mach_initialize.py

import math
import os
import shutil
import sys
from pathlib import Path

from importlib.abc import MetaPathFinder

from types import ModuleType


CATEGORIES = {
    'bootstrap': {
        'short': 'Bootstrap Commands',
        'long': 'Bootstrap the build system',
        'priority': 90,
    },
    'build': {
        'short': 'Build Commands',
        'long': 'Interact with the build system',
        'priority': 80,
    },
    'post-build': {
        'short': 'Post-build Commands',
        'long': 'Common actions performed after completing a build.',
        'priority': 70,
    },
    'testing': {
        'short': 'Testing',
        'long': 'Run tests.',
        'priority': 60,
    },
    'devenv': {
        'short': 'Development Environment',
        'long': 'Set up and configure your development environment.',
        'priority': 50,
    },
    'build-dev': {
        'short': 'Low-level Build System Interaction',
        'long': 'Interact with specific parts of the build system.',
        'priority': 20,
    },
    'package': {
        'short': 'Package',
        'long': 'Create objects to distribute',
        'priority': 15,
    },
    'misc': {
        'short': 'Potpourri',
        'long': 'Potent potables and assorted snacks.',
        'priority': 10,
    },
    'disabled': {
        'short': 'Disabled',
        'long': 'The disabled commands are hidden by default. Use -v to display them. These commands are unavailable '
                'for your current context, run "mach <command>" to see why.',
        'priority': 0,
    }
}


def _activate_python_environment(topsrcdir, get_state_dir):
    from mach.site import MachSiteManager

    mach_environment = MachSiteManager.from_environment(
        topsrcdir,
        get_state_dir,
    )
    mach_environment.activate()


def check_for_spaces(topsrcdir):
    if " " in topsrcdir:
        raise Exception(
            f"Your checkout at path '{topsrcdir}' contains a space, which "
            f"is not supported. Please move it to somewhere that does not "
            f"have a space in the path before rerunning mach."
        )


def initialize(topsrcdir, args=()):
    # We need the "mach" module to access the logic to parse virtualenv
    # requirements. Since that depends on "packaging", we add it to the path too.
    sys.path[0:0] = [
        os.path.join(topsrcdir, module)
        for module in (
            os.path.join("third_party", "mach"),
            #os.path.join("third_party", "python", "packaging"),
        )
    ]

    from mach.util import get_state_dir, get_virtualenv_base_dir, setenv

    state_dir = _create_state_dir(topsrcdir)

    check_for_spaces(topsrcdir)

    # normpath state_dir to normalize msys-style slashes.
    _activate_python_environment(
        topsrcdir, lambda: os.path.normpath(get_state_dir(True, topsrcdir=topsrcdir))
    )

    import mach.main
    from mach.command_util import (
        MACH_COMMANDS,
        DetermineCommandVenvAction,
        load_commands_from_spec,
    )
    from mach.main import get_argument_parser

    parser = get_argument_parser(
        action=DetermineCommandVenvAction,
        topsrcdir=topsrcdir,
    )
    namespace = parser.parse_args()

    command_name = getattr(namespace, "command_name", None)
    site_name = getattr(namespace, "site_name", "common")
    command_site_manager = None

    # the 'clobber' command needs to run in the 'mach' venv, so we
    # don't want to activate any other virtualenv for it.
    if command_name != "clobber" and command_name != "clean":
        from mach.site import CommandSiteManager

        command_site_manager = CommandSiteManager.from_environment(
            topsrcdir,
            lambda: os.path.normpath(get_state_dir(True, topsrcdir=topsrcdir)),
            site_name,
            get_virtualenv_base_dir(topsrcdir),
        )

        command_site_manager.activate()


    def resolve_repository():
        return None

    def pre_dispatch_handler(context, handler, args):
        return

    def post_dispatch_handler(
        context, handler, instance, success, start_time, end_time, depth, args
    ):
        """Perform global operations after command dispatch.


        For now,  we will use this to handle build system telemetry.
        """
        return

    def populate_context(key=None):
        if key is None:
            return
        if key == "state_dir":
            return state_dir

        if key == "local_state_dir":
            return get_state_dir(specific_to_topsrcdir=True)

        if key == "topdir":
            return topsrcdir

        if key == "pre_dispatch_handler":
            return pre_dispatch_handler

        if key == "post_dispatch_handler":
            return post_dispatch_handler

        if key == "repository":
            return resolve_repository()

        raise AttributeError(key)

    driver = mach.main.Mach(os.getcwd(), command_site_manager)
    driver.populate_context_handler = populate_context

    if not driver.settings_paths:
        # default global machrc location
        driver.settings_paths.append(state_dir)
    # always load local repository configuration
    driver.settings_paths.append(topsrcdir)

    for category, meta in CATEGORIES.items():
        driver.define_category(category, meta["short"], meta["long"], meta["priority"])

    # Sparse checkouts may not have all mach_commands.py files. Ignore
    # errors from missing files. Same for spidermonkey tarballs.
    repo = resolve_repository()
    if repo != "SOURCE":
        missing_ok = (
            repo is not None and repo.sparse_checkout_present()
        ) or os.path.exists(os.path.join(topsrcdir, "INSTALL"))
    else:
        missing_ok = ()

    commands_that_need_all_modules_loaded = [
        "busted",
        "help",
        "mach-commands",
        "mach-completion",
        "mach-debug-commands",
    ]

    def commands_to_load(top_level_command: str):
        visited = set()

        def find_downstream_commands_recursively(command: str):
            if not MACH_COMMANDS.get(command):
                return

            if command in visited:
                return

            visited.add(command)

            for command_dependency in MACH_COMMANDS[command].command_dependencies:
                find_downstream_commands_recursively(command_dependency)

        find_downstream_commands_recursively(top_level_command)

        return list(visited)

    if (
        command_name not in MACH_COMMANDS
        or command_name in commands_that_need_all_modules_loaded
    ):
        command_modules_to_load = MACH_COMMANDS
    else:
        command_names_to_load = commands_to_load(command_name)
        command_modules_to_load = {
            command_name: MACH_COMMANDS[command_name]
            for command_name in command_names_to_load
        }

    load_commands_from_spec(command_modules_to_load, topsrcdir, missing_ok=missing_ok)

    return driver


def _create_state_dir(topsrcdir):
    # Global build system and mach state is stored in a central directory.
    state_dir = os.path.join(topsrcdir, f"_machon{sys.version_info[0]}.{sys.version_info[1]}")
    if not os.path.exists(state_dir):
        print(
            "Creating global state directory from environment variable: {}".format(
                state_dir
            )
        )

    os.makedirs(state_dir, mode=0o770, exist_ok=True)
    return state_dir


# Hook import such that .pyc/.pyo files without a corresponding .py file in
# the source directory are essentially ignored. See further below for details
# and caveats.
# Objdirs outside the source directory are ignored because in most cases, if
# a .pyc/.pyo file exists there, a .py file will be next to it anyways.
class ImportHook(object):
    def __init__(self, original_import):
        self._original_import = original_import
        # Assume the source directory is the parent directory of the one
        # containing this file.
        self._source_dir = (
            os.path.normcase(
                os.path.abspath(os.path.dirname(os.path.dirname(__file__)))
            )
            + os.sep
        )
        self._modules = set()

    def __call__(self, name, globals=None, locals=None, fromlist=None, level=-1):
        if sys.version_info[0] >= 3 and level < 0:
            level = 0

        # name might be a relative import. Instead of figuring out what that
        # resolves to, which is complex, just rely on the real import.
        # Since we don't know the full module name, we can't check sys.modules,
        # so we need to keep track of which modules we've already seen to avoid
        # to stat() them again when they are imported multiple times.
        module = self._original_import(name, globals, locals, fromlist, level)

        # Some tests replace modules in sys.modules with non-module instances.
        if not isinstance(module, ModuleType):
            return module

        resolved_name = module.__name__
        if resolved_name in self._modules:
            return module
        self._modules.add(resolved_name)

        # Builtin modules don't have a __file__ attribute.
        if not getattr(module, "__file__", None):
            return module

        # Note: module.__file__ is not always absolute.
        path = os.path.normcase(os.path.abspath(module.__file__))
        # Note: we could avoid normcase and abspath above for non pyc/pyo
        # files, but those are actually rare, so it doesn't really matter.
        if not path.endswith((".pyc", ".pyo")):
            return module

        # Ignore modules outside our source directory
        if not path.startswith(self._source_dir):
            return module

        # If there is no .py corresponding to the .pyc/.pyo module we're
        # loading, remove the .pyc/.pyo file, and reload the module.
        # Since we already loaded the .pyc/.pyo module, if it had side
        # effects, they will have happened already, and loading the module
        # with the same name, from another directory may have the same side
        # effects (or different ones). We assume it's not a problem for the
        # python modules under our source directory (either because it
        # doesn't happen or because it doesn't matter).
        if not os.path.exists(module.__file__[:-1]):
            if os.path.exists(module.__file__):
                os.remove(module.__file__)
            del sys.modules[module.__name__]
            module = self(name, globals, locals, fromlist, level)

        return module


# Hook import such that .pyc/.pyo files without a corresponding .py file in
# the source directory are essentially ignored. See further below for details
# and caveats.
# Objdirs outside the source directory are ignored because in most cases, if
# a .pyc/.pyo file exists there, a .py file will be next to it anyways.
class FinderHook(MetaPathFinder):
    def __init__(self, klass):
        # Assume the source directory is the parent directory of the one
        # containing this file.
        self._source_dir = (
            os.path.normcase(
                os.path.abspath(os.path.dirname(os.path.dirname(__file__)))
            )
            + os.sep
        )
        self.finder_class = klass

    def find_spec(self, full_name, paths=None, target=None):
        spec = self.finder_class.find_spec(full_name, paths, target)

        # Some modules don't have an origin.
        if spec is None or spec.origin is None:
            return spec

        # Normalize the origin path.
        path = os.path.normcase(os.path.abspath(spec.origin))
        # Note: we could avoid normcase and abspath above for non pyc/pyo
        # files, but those are actually rare, so it doesn't really matter.
        if not path.endswith((".pyc", ".pyo")):
            return spec

        # Ignore modules outside our source directory
        if not path.startswith(self._source_dir):
            return spec

        # If there is no .py corresponding to the .pyc/.pyo module we're
        # resolving, remove the .pyc/.pyo file, and try again.
        if not os.path.exists(spec.origin[:-1]):
            if os.path.exists(spec.origin):
                os.remove(spec.origin)
            spec = self.finder_class.find_spec(full_name, paths, target)

        return spec


# Additional hook for python >= 3.8's importlib.metadata.
class MetadataHook(FinderHook):
    def find_distributions(self, *args, **kwargs):
        return self.finder_class.find_distributions(*args, **kwargs)


def hook(finder):
    has_find_spec = hasattr(finder, "find_spec")
    has_find_distributions = hasattr(finder, "find_distributions")
    if has_find_spec and has_find_distributions:
        return MetadataHook(finder)
    elif has_find_spec:
        return FinderHook(finder)
    return finder


# Install our hook. This can be deleted when the Python 3 migration is complete.
if sys.version_info[0] < 3:
    builtins.__import__ = ImportHook(builtins.__import__)
else:
    sys.meta_path = [hook(c) for c in sys.meta_path]