#!/usr/bin/env python3
import sys
from adam_reasoner import main
try:
    main(history_enabled=True)
except Exception as error:
    print(str(error),file=sys.stderr)
    sys.exit(1)
