- BITNESS = 16
- 1 << BITNESS words of RAM
- 8 registers
    - register 6 is program counter
    - register 7 is stack pointer
    - register 8 is instruction register

MOV    - 000001
ADD    - 000010
SUB    - 000011
MUL    - 000100
DIV    - 000101
CALL   - 000110
JE     - 000111
JNE    - 001000
JG     - 001001
JGE    - 001010
JL     - 001011
JLE    - 001100
PUSH   - 001101
POP    - 001110
OUT    - 001111
IN     - 010000
RET    - 010001
INT    - 010010
EOI    - 010011
INC    - 010100
DEC    - 010101

o - operation
s - source
d - destination
m - whether the source or destination is direct or an address in memory
oooooomssssmdddd

REG A     - 0001
REG B     - 0010
REG C     - 0011
REG D     - 0100
REG E     - 0101
REG PC    - 0110
REG SP    - 0111
REG INSTR - 1000
NEXT WORD - 1001
