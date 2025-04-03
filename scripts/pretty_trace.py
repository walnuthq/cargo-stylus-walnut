#!/usr/bin/env python3

import json
import sys
from pathlib import Path

try:
    import colorama
    from colorama import Fore, Style
except ImportError:
    print("Please install colorama via `pip install colorama`.")
    sys.exit(1)

def pretty_print_trace(trace_file: Path):
    """
    Reads a JSON array of function call frames from `trace_file`
    and prints them in a nice, colored backtrace format.
    """
    if not trace_file.is_file():
        print(f"ERROR: No such file: {trace_file}")
        sys.exit(1)

    with open(trace_file, "r") as f:
        try:
            call_list = json.load(f)
        except json.JSONDecodeError as e:
            print(f"ERROR: Failed to parse JSON: {e}")
            sys.exit(1)

    if not isinstance(call_list, list):
        print("ERROR: JSON root is not a list of frames.")
        sys.exit(1)

    # Print a header
    print(f"{Fore.CYAN}=== WALNUT FUNCTION BACKTRACE ==={Style.RESET_ALL}")

    for i, call_frame in enumerate(call_list, start=0):
        function_name = call_frame.get("function", "<unknown function>")
        file_name = call_frame.get("file", "<no file>")
        line = call_frame.get("line", 0)
        args = call_frame.get("args", [])

        # Print the frame header
        print(
            f"{Fore.GREEN}#{i:<2}{Style.RESET_ALL} "
            f"{Fore.YELLOW}{function_name}{Style.RESET_ALL} "
            f"({file_name}:{line})"
        )

        # Print arguments if present
        if args:
            for arg in args:
                arg_name = arg.get("name", "<arg>")
                arg_value = arg.get("value", "<unavailable>")
                print(f"      {Fore.MAGENTA}{arg_name}{Style.RESET_ALL} = {arg_value}")

        # Blank line after each frame
        print()

def main():
    colorama.init(autoreset=True)

    # Default to /tmp/lldb_function_trace.json if no arg given
    if len(sys.argv) > 1:
        trace_file = Path(sys.argv[1])
    else:
        trace_file = Path("/tmp/lldb_function_trace.json")

    pretty_print_trace(trace_file)

if __name__ == "__main__":
    main()
