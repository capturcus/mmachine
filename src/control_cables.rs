const REGISTERS_NUM: usize = 4;

pub struct ControlCables {
    pub halt: bool,
    pub memory_address_in: bool,
    pub ram_in: bool,
    pub ram_out: bool,
    pub intruction_register_out: bool,
    pub intruction_register_in: bool,
    pub reg_in: [bool; REGISTERS_NUM],
    pub reg_out: [bool; REGISTERS_NUM],
    pub add_mul: bool,
    pub sub_div: bool,
    pub counter_enable: bool,
    pub counter_out: bool,
    pub counter_in: bool,
    pub input_out: bool,
    pub output_in: bool,
}

impl ControlCables {
    pub fn new() -> ControlCables {
        ControlCables {
            halt: false,
            memory_address_in: false,
            ram_in: false,
            ram_out: false,
            intruction_register_out: false,
            intruction_register_in: false,
            reg_in: [false; REGISTERS_NUM],
            reg_out: [false; REGISTERS_NUM],
            add_mul: false,
            sub_div: false,
            counter_enable: false,
            counter_out: false,
            counter_in: false,
            input_out: false,
            output_in: false,
        }
    }

    pub fn reset(&mut self) {
        self.halt = false;
        self.memory_address_in = false;
        self.ram_in = false;
        self.ram_out = false;
        self.intruction_register_out = false;
        self.intruction_register_in = false;
        self.reg_in = [false; REGISTERS_NUM];
        self.reg_out = [false; REGISTERS_NUM];
        self.add_mul = false;
        self.sub_div = false;
        self.counter_enable = false;
        self.counter_out = false;
        self.counter_in = false;
        self.input_out = false;
        self.output_in = false;
    }
}
