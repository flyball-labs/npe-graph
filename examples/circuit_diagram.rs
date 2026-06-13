#![allow(dead_code)]
#![allow(unused_variables)]

use npe_graph::{Graph, NodeTemplate};

/// The data associated with an edge. Since this is a breadboard
/// circuit example there's no data but this could include wire
/// color, type, gauge, or any other edge design data.
#[derive(Clone, Debug)]
struct WireData;

/// The data associated with a pin. This enum can carry varying
/// levels of design data from a simple passive pin up to an
/// integrated circuit pinout
#[derive(Clone, Debug)]
enum PinData {
    /// Passive component pins are non-named and numbered
    Passive,
    /// Some simple components do have polar pins
    Polar(Polarity),
    /// IC pins also carry a description tag and an IC number
    Ic(IcPin),
}

impl PinData {
    fn passive() -> Self {
        PinData::Passive
    }

    fn ic(number: usize, description: &str) -> Self {
        PinData::Ic(IcPin {
            number,
            description: description.into(),
        })
    }

    fn polar(polarity: Polarity) -> Self {
        PinData::Polar(polarity)
    }

    fn polar_positive() -> Self {
        PinData::Polar(Polarity::Positive)
    }

    fn polar_negative() -> Self {
        PinData::Polar(Polarity::Negative)
    }
}

/// The polarity of a simple polar passive pin
#[derive(Clone, Debug)]
enum Polarity {
    Positive,
    Negative,
}

/// The data associated with a pin on an IC. This could be
/// expanded to include directionality, function, mapping to software
/// voltage/current limits, or anything else relevant to the design
#[derive(Clone, Debug)]
struct IcPin {
    number: usize,
    description: String,
}

impl IcPin {
    fn new(number: usize, description: &str) -> Self {
        Self {
            number,
            description: String::from(description),
        }
    }
}

/// The data associated with an integrated circuit. This could also
/// included manufacturer, suppliers, datasheet data, or rendering info
#[derive(Clone, Debug)]
struct IcData {
    name: String,
}

/// The data associated with a simple passive component. In this example
/// the type of the passive is marked by the enum but this struct could
/// also contain that so it's the single data type required for rendering
/// the correct circuit symbol.
#[derive(Clone, Debug)]
struct PassiveData {
    name: String,
    value: usize,
    unit: String,
}

/// The data associated with a node on the graph, split by the types
/// of nodes available.
#[derive(Clone, Debug)]
enum NodeData {
    Ic(IcData),
    Resistor(PassiveData),
    Capacitor(PassiveData),
    Inductor(PassiveData),
    VdcSource(usize),
    Label(String),
    Ground,
}

/// A net is a single port component that connects to some abstract
/// net in the circuit. This can include voltage sources, ground planes
/// or labelled nets like an output or input
#[derive(Clone, Debug)]
struct Net {
    data: NodeData,
    port: PinData,
}

impl Net {
    fn dc_source(name: &str, volts: usize) -> Self {
        Self {
            data: NodeData::VdcSource(volts),
            port: PinData::Passive,
        }
    }

    fn ground(name: &str) -> Self {
        Self {
            data: NodeData::Ground,
            port: PinData::Passive,
        }
    }

    fn label(name: &str) -> Self {
        Self {
            data: NodeData::Label(name.into()),
            port: PinData::Passive,
        }
    }
}

impl NodeTemplate<NodeData, PinData> for Net {
    fn node_data(&self) -> NodeData {
        self.data.clone()
    }

    fn port_data(&self) -> Vec<PinData> {
        vec![self.port.clone()]
    }
}

/// The data associated with a simple passive component. These
/// components are required to be a non-active two-pin component
/// that is optionally polar (per the variant of `PinData`)
#[derive(Clone, Debug)]
struct Passive {
    data: NodeData,
    ports: [PinData; 2],
}

impl Passive {
    fn capacitor(name: &str, value: usize) -> Self {
        Passive {
            data: NodeData::Capacitor(PassiveData {
                name: name.into(),
                value,
                unit: "uF".into(),
            }),
            ports: [PinData::passive(), PinData::passive()],
        }
    }

    fn resistor(name: &str, value: usize) -> Self {
        Passive {
            data: NodeData::Resistor(PassiveData {
                name: name.into(),
                value,
                unit: "ohm".into(),
            }),
            ports: [PinData::passive(), PinData::passive()],
        }
    }

    fn inductor(name: &str, value: usize) -> Self {
        Passive {
            data: NodeData::Inductor(PassiveData {
                name: name.into(),
                value,
                unit: "mh".into(),
            }),
            ports: [PinData::passive(), PinData::passive()],
        }
    }
}

impl NodeTemplate<NodeData, PinData> for Passive {
    fn node_data(&self) -> NodeData {
        self.data.clone()
    }

    fn port_data(&self) -> Vec<PinData> {
        self.ports.clone().into()
    }
}

struct LM555 {
    data: NodeData,
    ports: Vec<PinData>,
}

impl LM555 {
    fn default() -> Self {
        let ports = vec![
            PinData::ic(1, "GND"),
            PinData::ic(2, "Trigger"),
            PinData::ic(3, "Output"),
            PinData::ic(4, "Reset"),
            PinData::ic(5, "Control_Voltage"),
            PinData::ic(6, "Threshold"),
            PinData::ic(7, "Discharge"),
            PinData::ic(8, "V_Plus"),
        ];
        Self {
            data: NodeData::Ic(IcData {
                name: String::from("555 timer"),
            }),
            ports,
        }
    }
}

impl NodeTemplate<NodeData, PinData> for LM555 {
    fn node_data(&self) -> NodeData {
        self.data.clone()
    }

    fn port_data(&self) -> Vec<PinData> {
        self.ports.clone()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Instantiate the graph with the correct data types
    let mut g: Graph<NodeData, PinData, WireData> = Graph::new();

    // Add the components by their templates, which automatically
    // creates the nodes, adds its node data, then creates the ports
    // with their node data
    let (lm555, lm555_ports) = g.instantiate(&LM555::default());
    let (c1, c1_ports) = g.instantiate(&Passive::capacitor("C1", 10));
    let (r1, r1_ports) = g.instantiate(&Passive::resistor("R1", 100));
    let (r2, r2_ports) = g.instantiate(&Passive::resistor("R2", 100));
    let (filter_cap, filter_cap_ports) = g.instantiate(&Passive::capacitor("F_CAP", 100));

    let (vsource, vsource_ports) = g.instantiate(&Net::dc_source("VDC", 5));
    let (gnd, gnd_ports) = g.instantiate(&Net::ground("GND"));
    let (output, output_ports) = g.instantiate(&Net::label("OUTPUT"));

    // Wire the circuit up
    // g.connect(lm555_ports.iter().filter(|p| p))

    Ok(())
}
