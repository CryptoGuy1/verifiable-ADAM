#!/usr/bin/env python3
"""Read-only deployment inventory. Does not probe sensors, install or change services."""
import datetime
import glob
import json
import os
import platform
import shutil
import subprocess
import time

def command(args):
    if not shutil.which(args[0]):
        return {"available":False}
    try:
        result=subprocess.run(args,capture_output=True,text=True,timeout=5)
        return {"available":True,"exit_code":result.returncode,"stdout":result.stdout[:8192].strip(),"stderr":result.stderr[:1000].strip()}
    except Exception as error:
        return {"available":True,"error":str(error)}

def main():
    try:
        with open("/proc/device-tree/model","rb") as source:
            model=source.read(512).decode().strip("\0\n")
    except OSError:
        model=None
    result={"captured_at_utc":datetime.datetime.now(datetime.timezone.utc).isoformat(),"monotonic_seconds":time.monotonic(),"hostname":platform.node(),"architecture":platform.machine(),"model":model,"is_raspberry_pi":bool(model and "Raspberry Pi" in model),"i2c_device_files":sorted(glob.glob("/dev/i2c-*")),"sensor_acquisition_tested":False,"services_changed":False}
    for name,args in {
        "os":["cat","/etc/os-release"],"kernel":["uname","-a"],"cpu":["lscpu"],
        "memory":["free","-h"],"storage":["df","-h","/"],
        "temperature":["vcgencmd","measure_temp"],"power_thermal_flags":["vcgencmd","get_throttled"],
        "clock_sync":["timedatectl","show","-p","NTPSynchronized","-p","Timezone"],
        "rust":["rustc","--version"],"cargo":["cargo","--version"],"python":["python3","--version"],
    }.items():
        result[name]=command(args)
    print(json.dumps(result,indent=2))

if __name__=="__main__":
    main()
