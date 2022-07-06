import os

output_path = r"C:\Users\larsw\Documents\Workspaces\DAR\Dial-A-Ride\results\outputs"
export_path = r"C:\Users\larsw\Documents\Workspaces\DAR\Dial-A-Ride\results\exports"


# parse the results
class Result:
    def __init__(self, filename):
        split_file = filename.split("_")
        self.fp = int(split_file[0])
        self.ep = int(split_file[1])
        self.ed = int(split_file[2])
        self.afs = int(split_file[3])
        self.sample = int(split_file[4])
        self.scenario = int(split_file[5])
        self.optimal = split_file[6] == "optimal.txt"
        with open(os.path.join(output_path, filename), "r") as f:
            content = f.read()
        if content == "TIMEOUT":
            self.distance = -1
            self.time = -1
        else:
            solution_found = False
            for line in content.split("\n"):
                if line.startswith("Found optimal solution"):
                    solution_found = True
                    line = line[48:-30].split(" ")
                    self.distance = int(line[0])
                    self.time = int(line[2])
                    break
            if not solution_found:
                print(f"Could not read {filename}")


results = [Result(filename) for filename in os.listdir(output_path)]
print(f"Parsed {len(results)} results")

# group the results


class Group:
    class ScenarioGroup:
        def __init__(self):
            self.optimal_distance = {}
            self.nonoptimal_distance = {}
            self.optimal_time = {}
            self.nonoptimal_time = {}

        def add_result(self, result):
            if result.optimal:
                self.optimal_distance[result.sample] = result.distance
                self.optimal_time[result.sample] = result.time
            else:
                self.nonoptimal_distance[result.sample] = result.distance
                self.nonoptimal_time[result.sample] = result.time

    def __init__(self, result):
        self.fp = result.fp
        self.ep = result.ep
        self.ed = result.ed
        self.afs = result.afs
        self.scenarios = [Group.ScenarioGroup() for _ in range(6)]

    def add_result(self, result):
        assert(self.right_group(result))
        self.scenarios[result.scenario-1].add_result(result)

    def right_group(self, result):
        return self.fp == result.fp and self.ep == result.ep and self.ed == result.ed and self.afs == result.afs

    def is_type_a(self):
        return 2*self.fp > self.ep + self.ed

    def get_table_value(self, scenario, optimal, sample, distance):
        base = self.scenarios[scenario - 1]
        if optimal:
            if distance:
                if sample in base.optimal_distance:
                    value = base.optimal_distance[sample]
                    if value == -1:
                        return "TO"
                    return value
            else:
                if sample in base.optimal_time:
                    value = base.optimal_time[sample]
                    if value == -1:
                        return "TO"
                    return value
        else:
            if distance:
                if sample in base.nonoptimal_distance:
                    value = base.nonoptimal_distance[sample]
                    if value == -1:
                        return "TO"
                    return value
            else:
                if sample in base.nonoptimal_time:
                    value = base.nonoptimal_time[sample]
                    if value == -1:
                        return "TO"
                    return value
        return "NA"

    def get_time_summary_string(self, scenario):
        base = self.scenarios[scenario - 1]
        optimal_time = 0
        for sample in range(1, 6):
            if sample in base.optimal_time:
                if base.optimal_time[sample] == -1:
                    optimal_time = "TO"
                    break
                else:
                    optimal_time += base.optimal_time[sample]
            else:
                optimal_time = "NA"
                break
        if not isinstance(optimal_time, str):
            optimal_time //= 5
        nonoptimal_time = 0
        for sample in range(1, 6):
            if sample in base.nonoptimal_time:
                if base.nonoptimal_time[sample] == -1:
                    nonoptimal_time = "TO"
                    break
                else:
                    nonoptimal_time += base.nonoptimal_time[sample]
            else:
                nonoptimal_time = "NA"
                break
        if not isinstance(nonoptimal_time, str):
            nonoptimal_time //= 5
        return f" & {optimal_time} & {nonoptimal_time}"


current_group = Group(results[0])
grouped_results = [current_group]
for result in results:
    if not current_group.right_group(result):
        current_group = Group(result)
        grouped_results.append(current_group)
    current_group.add_result(result)

grouped_a_results = [group for group in grouped_results if group.is_type_a()]
grouped_b_results = [
    group for group in grouped_results if not group.is_type_a()]


def create_detailed_table(grouped_results, distance):
    grouped_results = sorted(
        grouped_results, key=lambda g: 10*(2 * g.fp + g.ep + g.ed) + g.afs)
    data_type = "a" if grouped_results[0].is_type_a() else "b"
    short_value = "distance" if distance else "time"
    table_string = "\\begin{longtable}{|c|c|c|c|c|c c|c c|c c|c c|c c|}\n\\caption{" + \
        ("Distances" if distance else "Calculation times") + " of optimal routes for " + data_type.upper() + "-type data}\n" + \
        "\\label{tab:" + short_value + \
        "_" + data_type + \
        "} \\\\\n\\hline\n" + "FP  & EP & ED & AFS & S"
    for sample in range(1, 6):
        table_string += f" & $o_{sample}$ & $n_{sample}$"
    table_string += "\\\\\n\\hline\\hline"
    for group in grouped_results:
        for scenario in range(1, 7):
            if scenario == 1:
                values = [group.fp, group.ep, group.ed, group.afs]
                for value in values:
                    table_string += "\\multirow{6}{*}{"+str(value)+"} & "
            else:
                table_string += " & & & & "
            table_string += f"{scenario} "
            for sample in range(1, 6):
                table_string += f"& {group.get_table_value(scenario, True, sample, distance)} & {group.get_table_value(scenario, False, sample, distance)} "
            table_string += "\\\\\n"
        table_string += "\\hline\n"
    table_string += "\\end{longtable}"
    filename = f"{short_value}_{data_type}_table.tex"
    with open(os.path.join(export_path, filename), "w") as f:
        f.write(table_string)


create_detailed_table(grouped_a_results, True)
create_detailed_table(grouped_a_results, False)
create_detailed_table(grouped_b_results, True)
create_detailed_table(grouped_b_results, False)


def create_summary_table(grouped_results):
    grouped_results = sorted(
        grouped_results, key=lambda g: 10*(2 * g.fp + g.ep + g.ed) + g.afs)
    data_type = "a" if grouped_results[0].is_type_a() else "b"
    table_string = "\\begin{longtable}{|c|c|c|c|c c|c c|c c|c c|c c|c c|}\n" \
        + "\\caption{Average calculation time of optimal routes for " \
        + data_type.upper() + "-type data}\n" \
        + "\\label{tab:avg_time_" + data_type + "} \\\\\n" \
        + "\\hline\n FP & EP & ED & AFS & $S^o_1$ & $S^n_1$ & $S^o_2$ & $S^n_2$ & $S^o_3$ & $S^n_3$ & $S^o_4$ & $S^n_4$ & $S^o_5$ & $S^n_5$ & $S^o_6$ & $S^n_6$ \\\\" \
        + "\n\\hline\\hline\n"
    for group in grouped_results:
        table_string += f"{group.fp} & {group.ep} & {group.ed} & {group.afs}"
        for scenario in range(1, 7):
            table_string += group.get_time_summary_string(scenario)
        table_string += " \\\\\n\\hline\n"
    table_string += "\\end{longtable}"
    filename = f"time_summary_{data_type}.tex"
    with open(os.path.join(export_path, filename), "w") as f:
        f.write(table_string)


create_summary_table(grouped_a_results)
create_summary_table(grouped_b_results)
