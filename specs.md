- BITNESS = 16
- 1 << BITNESS words of RAM
- 8 registers
    - register 6 is program counter
    - register 7 is stack pointer
    - register 8 is instruction register

HLT    - 000000
stops the cpu clock

MOV    - 000001
moves from src to dst

ADD    - 000010
adds values in a and b and stores the result in dst

SUB    - 000011
subs b from a and stores the result in dst

MUL    - 000100
multiplies a and b and stores the result in dst

DIV    - 000101
int divides a and b and stores the result in dst

CALL   - 000110
pushes pc to stack and jumps to dst

JE     - 000111
jumps to dst if a and b are equal

JNE    - 001000
jumps to dst if a and b are not equal

JG     - 001001
jumps to dst if a is greater than b

JGE    - 001010
jumps to dst if a is greater or equal than b

JL     - 001011
jumps to dst if a is less than b

JLE    - 001100
jumps to dst if a is less or equal than b

PUSH   - 001101
pushes src to the stack

POP    - 001110
pops from stack to dst

OUT    - 001111
writes an output value, src is port and dst is value

IN     - 010000
reads a value from input, src is port dst is value

INT    - 010010
trigger a software interrupt with value src

EOI    - 010011
pops a return address from the stack and jumps there, enables interrupts

INC    - 010100
increment dst by 1

DEC    - 010101
decrement dst by 1

LOAD   - 010110
load value from address src to dst

STORE  - 010111
store value from src to address dst

LDCNST - 011000
load a constant from the executable to dst

o - operation
s - source
d - destination
oooooosssssddddd

registers

REG A    - 00000 (left input to alu)
REG B    - 00001 (right input to alu)
REG C    - 00010
REG D    - 00011
REG E    - 00100
REG PC   - 00101
REG SP   - 00110
REG INST - 00111
