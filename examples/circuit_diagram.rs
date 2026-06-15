//! This example constructs a basic circuit diagram of a 555 timer.
//! It's an example of how a circuit diagram might be constructed from a
//! node-port-edge graph, where the nodes are components, the ports are
//! pins, and the edges are wires. There are many valid data structures here;
//! this is merely one interesting example.
//!
//!  ```
//!                            ^                 
//!                           ^^^  VCC           
//!                          ^^^^^               
//!                            │                 
//!       ┌────────────────────┤                 
//!       │                    │                 
//!       │                    │                 
//!      ┌┴┐             ┌─────┴────┐            
//!   R1 │ │            8│         4│            
//!      │ │        ┌────┴──────────┴────┐       
//!      └┬┘       7│    VCC        R    │       
//!       │  ┌──────┤DIS                 │       
//!       │  │      │                 OUT├──────►
//!       ├──┘     6│        NE555       │3      
//!       │    ┌────┤THR                 │       
//!      ┌┴┐   │    │                    │       
//!   R2 │ │   ├────┤TR     GND       CV │       
//!      │ │   │   2└────────┬─────────┬─┘       
//!      └┬┘   │            1│        5│         
//!       │    │             │         │         
//!       ├────┘             │         │         
//!       │                  │       ──┴──  0.1uF
//!  C1 ──┴──                │       ──┬──  F_CAP
//!     ──┬──                │         │         
//!       └──────────────┬───┴─────────┘         
//!                      │                       
//!                   ───┴───                    
//!                    ─────  GND                
//!                     ───                      
//!  ```

#![allow(dead_code)]

use npe_graph::{Graph, KeyedNodeTemplate, NodeTemplate};

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
    /// IC pins also carry a description tag and an package pin number
    Ic(IcPin),
}

impl PinData {
    fn new_passive() -> Self {
        PinData::Passive
    }

    fn new_ic(number: usize, description: &str) -> Self {
        PinData::Ic(IcPin {
            number,
            description: description.into(),
        })
    }

    fn new_polar(polarity: Polarity) -> Self {
        PinData::Polar(polarity)
    }

    fn new_polar_positive() -> Self {
        PinData::Polar(Polarity::Positive)
    }

    fn new_polar_negative() -> Self {
        PinData::Polar(Polarity::Negative)
    }

    /// The human-facing name of a pin, if it has one
    fn name(&self) -> Option<&str> {
        match self {
            PinData::Ic(ic) => Some(&ic.description),
            _ => None,
        }
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
/// also contain the type so it's this single data type required for rendering
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
enum ComponentData {
    Ic(IcData),
    Resistor(PassiveData),
    Capacitor(PassiveData),
    Inductor(PassiveData),
    VdcSource(String, usize),
    Label(String),
    Ground(String),
}

/// A net is a single port component that connects to some abstract
/// net in the circuit. This can include voltage sources, ground planes
/// or labelled nets like an output or input
#[derive(Clone, Debug)]
struct Net {
    data: ComponentData,
    port: PinData,
}

impl Net {
    fn new_dc_source(name: &str, volts: usize) -> Self {
        Self {
            data: ComponentData::VdcSource(name.into(), volts),
            port: PinData::Passive,
        }
    }

    fn new_ground(name: &str) -> Self {
        Self {
            data: ComponentData::Ground(name.into()),
            port: PinData::Passive,
        }
    }

    fn new_label(name: &str) -> Self {
        Self {
            data: ComponentData::Label(name.into()),
            port: PinData::Passive,
        }
    }
}

/// Implement the `NodeTemplate` trait for the `Net` so that
/// it can be created with an `instantiate` call
impl NodeTemplate<ComponentData, PinData> for Net {
    fn node_data(&self) -> ComponentData {
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
    data: ComponentData,
    ports: [PinData; 2],
}

impl Passive {
    fn new_capacitor(name: &str, value: usize) -> Self {
        Passive {
            data: ComponentData::Capacitor(PassiveData {
                name: name.into(),
                value,
                unit: "uF".into(),
            }),
            ports: [PinData::new_passive(), PinData::new_passive()],
        }
    }

    fn new_resistor(name: &str, value: usize) -> Self {
        Passive {
            data: ComponentData::Resistor(PassiveData {
                name: name.into(),
                value,
                unit: "ohm".into(),
            }),
            ports: [PinData::new_passive(), PinData::new_passive()],
        }
    }

    fn new_inductor(name: &str, value: usize) -> Self {
        Passive {
            data: ComponentData::Inductor(PassiveData {
                name: name.into(),
                value,
                unit: "mh".into(),
            }),
            ports: [PinData::new_passive(), PinData::new_passive()],
        }
    }
}

/// Implement the `NodeTemplate` trait for the `Passive` so that
/// it can be created with an `instantiate` call
impl NodeTemplate<ComponentData, PinData> for Passive {
    fn node_data(&self) -> ComponentData {
        self.data.clone()
    }

    fn port_data(&self) -> Vec<PinData> {
        self.ports.clone().into()
    }
}

/// Because `npe_graph` stores the ports separate from the node data
/// a component can't just be created from a template by passing the
/// data object to the graph. It requires a template object to know
/// how to construct it.
/// This struct is a template for an LM555 timer chip. It stores the
/// data that should live on the node and the ports that should be
/// attached to it. The `Graph` then knows how to instantiate it.
struct Lm555 {
    data: ComponentData,
    ports: Vec<PinData>,
}

impl Lm555 {
    fn default() -> Self {
        let ports = vec![
            PinData::new_ic(1, "Ground"),
            PinData::new_ic(2, "Trigger"),
            PinData::new_ic(3, "Output"),
            PinData::new_ic(4, "Reset"),
            PinData::new_ic(5, "Control_Voltage"),
            PinData::new_ic(6, "Threshold"),
            PinData::new_ic(7, "Discharge"),
            PinData::new_ic(8, "V_Plus"),
        ];
        Self {
            data: ComponentData::Ic(IcData {
                name: String::from("NE555"),
            }),
            ports,
        }
    }
}

/// This trait implements a keyed node template. It defines not
/// just the node data and ports but a set of keys associated with
/// each pin. When the node is instantiated on the graph the constructor
/// function returns a `HashMap` with the passed in keys and the `PortId`s
/// that were constructed. This makes it much easier to find the correct
/// `PortId`s for a given port without having to query the `Graph` to figure
/// out what `PortId` correlates with which logical port.
/// This isn't usually an issue in a GUI app since the port/pin data is
/// being rendered on the visual port. So dragging an edge to that port will
/// just directly connect the correct ID.
impl KeyedNodeTemplate<ComponentData, PinData, String> for Lm555 {
    fn node_data(&self) -> ComponentData {
        self.data.clone()
    }

    /// Use the pin names as port keys for this chip since
    /// the pin names are unique
    fn keyed_ports(&self) -> Vec<(String, PinData)> {
        self.ports
            .iter()
            .map(|p| (p.name().unwrap_or("unnamed").into(), p.clone()))
            .collect()
    }
}

/// Also implement the non-keyed version for comparison
impl NodeTemplate<ComponentData, PinData> for Lm555 {
    fn node_data(&self) -> ComponentData {
        self.data.clone()
    }

    fn port_data(&self) -> Vec<PinData> {
        self.ports.clone()
    }
}

/// A convenient type alias for the specific type of graph
/// being instantiated is recommended, especially if the project
/// contains multiple graphs
type CircuitGraph = Graph<ComponentData, PinData, WireData>;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Instantiate the graph with the correct data types
    let mut g: CircuitGraph = Graph::new();

    // Add the components by their templates, which automatically
    // creates the nodes, adds its node data, then creates the ports
    // with their node data

    // The keyed method returns a HashMap of the PortIds with identifying
    // names for the pins. This isn't usually necessary in a GUI application
    // since the pins are being identified by the rendering of the diagram.
    // But for an API-first use like this it's more convenient than looking
    // up the PortIds in the graph to get their data to filter for a pin name.
    let (_lm555, lm555_pins) = g.instantiate_keyed(&Lm555::default());

    // The rest of the components only have one or two ports so they're
    // instantiated the simple way and return their NodeId and a bare vec
    // of PortIds.
    let (_c1, c1_ports) = g.instantiate(&Passive::new_capacitor("C1", 10));
    let (_r1, r1_ports) = g.instantiate(&Passive::new_resistor("R1", 100));
    let (_r2, r2_ports) = g.instantiate(&Passive::new_resistor("R2", 100));
    let (_filter_cap, filter_cap_ports) = g.instantiate(&Passive::new_capacitor("F_CAP", 100));

    let (_vsource, vsource_ports) = g.instantiate(&Net::new_dc_source("VDC", 5));
    let (_gnd, gnd_ports) = g.instantiate(&Net::new_ground("GND"));
    let (_output, output_ports) = g.instantiate(&Net::new_label("OUTPUT"));

    // Wire the circuit up
    // The LM555 pins can be referred to with a hashed name since they were a keyed
    // instantiation but the rest of the components use plain vector indexing.
    g.connect(lm555_pins["Ground"], gnd_ports[0], WireData)?;
    g.connect(filter_cap_ports[0], gnd_ports[0], WireData)?;

    g.connect(c1_ports[0], gnd_ports[0], WireData)?;
    g.connect(c1_ports[1], r2_ports[0], WireData)?;
    g.connect(r2_ports[1], r1_ports[0], WireData)?;
    g.connect(r1_ports[1], vsource_ports[0], WireData)?;

    g.connect(lm555_pins["Control_Voltage"], filter_cap_ports[1], WireData)?;
    g.connect(lm555_pins["Trigger"], c1_ports[1], WireData)?;
    g.connect(lm555_pins["Threshold"], c1_ports[1], WireData)?;
    g.connect(lm555_pins["Discharge"], r2_ports[1], WireData)?;
    g.connect(lm555_pins["V_Plus"], vsource_ports[0], WireData)?;
    g.connect(lm555_pins["Reset"], vsource_ports[0], WireData)?;
    g.connect(lm555_pins["Output"], output_ports[0], WireData)?;

    Ok(())
}
