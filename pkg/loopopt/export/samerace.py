from model import PROG as PROG

for program in PROG.values():
    program.race = {1025: 1012, 1027: 1006, 1028: 1005, 1029: 1016}.get(program.race, program.race)
