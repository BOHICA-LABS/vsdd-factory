import os, sys, time

outfile, delay = sys.argv[1], float(sys.argv[2])
pid = os.fork()
if pid == 0:
    os.setsid()
    pid2 = os.fork()
    if pid2 == 0:
        time.sleep(delay)
        with open(outfile, "w") as fh:
            fh.write("DETACHED-WROTE\n")
        os._exit(0)
    os._exit(0)
os.waitpid(pid, 0)
sys.exit(0)
