use svg::{
    node::element::{
        path::{
            Position,
            Command,
            Data,
        },
    },
    parser::Event,
};
use crate::Vec2;


pub fn svg_to_vector(source:&str)->Option<Vec<Vec2>> {
    if let Ok(events)=svg::read(source) {
        let mut vector=Vec::new();
        let mut cursor=Vec2::zero();
        let mut first=None;
        for event in events {
            match event {
                Event::Tag(_,_,attributes)=>{
                    if let Some(data)=attributes.get("d") {
                        if let Ok(data)=Data::parse(data) {
                            for command in data.iter() {
                                match &command {
                                    &Command::Move(pos,params)=>{
                                        match pos {
                                            Position::Absolute=>{
                                                cursor=Vec2::new(params[0],params[1]);
                                            },
                                            Position::Relative=>{
                                                cursor+=Vec2::new(params[0],params[1]);
                                            },
                                        }
                                    },
                                    &Command::Line(pos,params)=>{
                                        if let None=first {
                                            first=Some(cursor);
                                        }
                                        vector.push(cursor);
                                        match pos {
                                            Position::Absolute=>{
                                                cursor=Vec2::new(params[0],params[1]);
                                            },
                                            Position::Relative=>{
                                                cursor+=Vec2::new(params[0],params[1]);
                                            },
                                        }
                                        vector.push(cursor);
                                    },
                                    _=>{},
                                }
                            }
                        }
                    }
                }
                _=>{},
            }
        }
        if first.is_some() {
            vector.push(first.unwrap());
            vector.push(cursor);
        }
        assert!(vector.len()%2==0);
        Some(vector)
    } else {
        None
    }
}
