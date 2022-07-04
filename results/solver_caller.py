import os
import subprocess

dar_base_path = r"C:\Users\larsw\Documents\Workspaces\DAR\Dial-A-Ride\data"
output_path = r"C:\Users\larsw\Documents\Workspaces\DAR\Dial-A-Ride\results\outputs"
binary_path = r"C:\Users\larsw\Documents\Workspaces\DAR\Dial-A-Ride\target\release\dial_a_ride.exe"

env = os.environ.copy()
env["DAR_BASE_PATH"] = dar_base_path


class DataGroup:
    def __init__(self, file):
        self.fp = int(file.split("f")[0])
        self.ep = int(file.split("f")[1].split("p")[0])
        self.ed = int(file.split("p")[1].split("d")[0])
        self.afs = int(file.split("d")[1].split("s")[0])

    def num_nodes(self):
        return 2*self.fp + self.ep+self.ed


data_groups = sorted([DataGroup(file) for file in os.listdir(
    dar_base_path)], key=lambda o: o.num_nodes())


def blacklisted(fp, ep, ed, afs, sample, scenario, optimal):
    if fp >= 4 and ep >= 5 and ed >= 3 and afs >= 2 and scenario in [1, 2, 3] and optimal:
        return True
    elif fp >= 5 and ep >= 6 and ed >= 4 and scenario in [1, 2, 3]:
        return True
    elif fp >= 6 and ep >= 2 and ed >= 2 and afs >= 2 and scenario in [1, 2, 3] and optimal:
        return True
    elif fp >= 8 and ep >= 2 and ed >= 2 and scenario in [1, 2, 3]:
        return True
    elif fp == 13 and ep == 4 and ed == 2:
        return True
    elif fp == 8 and ep == 10 and ed == 6:
        return True
    elif fp >= 7 and ep >= 8 and ed >= 6 and scenario in [4, 5, 6] and optimal:
        return True
    else:
        return False


for group in data_groups:
    for sample in range(1, 6):
        for scenario in range(1, 7):
            for optimal in [True, False]:
                filename = f"{group.fp}_{group.ep}_{group.ed}_{group.afs}_{sample}_{scenario}_{'optimal' if optimal else 'nonoptimal'}.txt"
                out_path = os.path.join(output_path, filename)
                if not os.path.exists(out_path):
                    if not blacklisted(group.fp, group.ep, group.ed, group.afs, sample, scenario, optimal):
                        print(f"Calculating {filename}")
                        cmd = [binary_path, "--verbose"]
                        if not optimal:
                            cmd.append("--nonoptimal")
                        cmd += [str(group.fp), str(group.ep), str(group.ed), str(group.afs),
                                str(sample), str(scenario)]
                        p = subprocess.Popen(
                            cmd, stdout=subprocess.PIPE, env=env)
                        try:
                            out = str(p.communicate(timeout=3600)
                                      [0]).replace("\\n", "\n")[2:-1]
                            print(f"Completed {filename}")
                        except subprocess.TimeoutExpired:
                            out = "TIMEOUT"
                            print(f"Timeout for {filename}")
                        with open(out_path, "w") as f:
                            f.write(out)
                        p.terminate()
                    else:
                        print(f"{filename} was blacklisted")
                else:
                    print(f"{filename} already exists")
