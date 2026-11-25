import subprocess, sys, time
outfile, delay, block = sys.argv[1], float(sys.argv[2]), float(sys.argv[3])
subprocess.run(["python3", sys.argv[0].replace("spawncall.py", "spawn.py"), outfile, str(delay)], check=True)
time.sleep(block)
print("CALL-END", flush=True)
