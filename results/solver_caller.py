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

    def __lt__(self, other):
        if self.num_nodes() < other.num_nodes():
            return True
        elif other.num_nodes() > self.num_nodes():
            return False
        else:
            return self.afs < other.afs


data_groups = sorted([DataGroup(file) for file in os.listdir(dar_base_path)])

for group in data_groups:
    for sample in range(1, 6):
        for scenario in range(1, 7):
            for optimal in [True, False]:
                filename = f"{group.fp}_{group.ep}_{group.ed}_{group.afs}_{sample}_{scenario}_{'optimal' if optimal else 'nonoptimal'}.txt"
                out_path = os.path.join(output_path, filename)
                if not os.path.exists(out_path):
                    print(f"Calculating {filename}")
                    cmd = [binary_path, "--verbose"]
                    if not optimal:
                        cmd.append("--nonoptimal")
                    cmd += [str(group.fp), str(group.ep), str(group.ed), str(group.afs),
                            str(sample), str(scenario)]
                    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, env=env)
                    try:
                        out = str(p.communicate(timeout=3600)
                                  [0]).replace("\\n", "\n")[2:-1]
                        print(f"Completed {filename}")
                    except subprocess.TimeoutExpired:
                        out = "TIMEOUT"
                        print(f"Timeout for {filename}")
                    with open(out_path, "w") as f:
                        f.write(out)
                else:
                    print(f"{filename} already exists")
