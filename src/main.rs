//! Asteroid vectors created by me. Link: https://vectr.com/design/editor/4af10ec0-ce81-4622-8e33-72842af687f0


use minifb::{
    Window,
    Key,
    MouseMode,
    MouseButton,
};
use euc::{
    buffer::Buffer2d,
    rasterizer::Lines,
    Target,
    Pipeline,
};
use rand::{
    rngs::ThreadRng,
    Rng,
    thread_rng,
};
use embedded_graphics::{
    geometry::{
        Dimensions,
        Size,
        Point,
    },
    primitives::{
        PrimitiveStyleBuilder,
        Rectangle,
    },
    text::{
        Text,
        TextStyle as EgTextStyle,
        Alignment,
    },
    draw_target::DrawTarget,
    pixelcolor::BinaryColor,
    prelude::*,
    Pixel,
};
use bitmap_font::{
    tamzen::FONT_10x20,
    TextStyle,
};
use rodio::{
    source::Buffered,
    OutputStream,
    OutputStreamHandle,
    Sink,
    Decoder,
    Source,
};
use std::{
    time::{
        Duration,
        Instant,
    },
    ops::{
        Range,
        Deref,
        DerefMut,
    },
    fs::{
        read_to_string,
        read_dir,
        File,
    },
    io::BufReader,
    collections::HashMap,
};
use svg_to_vector::*;


mod svg_to_vector;


pub type Vec2=vek::Vec2<f32>;


const FPS:usize=30;
const DISABLE_GAME_OVER:bool=false;
const PLAYER_ACCEL:f32=600.0;        // m/s/s
const PLAYER_ROTATE_VEL:f32=7.0;     // deg/s
const PLAYER_RADIUS:f32=10.0;
const BULLET_VEL:f32=1000.0;
const BULLET_DELAY:Duration=Duration::from_millis(100);
const KEYMAP:KeyMap=KeyMap {
    forward:Key::W,
    backward:Key::S,
    left:Key::A,
    right:Key::D,
    fire:Key::F,
};
const MAX_ASTEROIDS:usize=50;
const ASTEROID_MAX_RADIUS:f32=80.0;
const ASTEROID_MIN_RADIUS:f32=30.0;
const NEW_ASTEROID_MIN_RADIUS:f32=40.0;
const PLAYER_VECTOR:&[Vec2]=&[
    Vec2::new(0.0,-0.02),
    Vec2::new(0.01,0.01),
    Vec2::new(0.0,-0.02),
    Vec2::new(-0.01,0.01),
    Vec2::new(-0.01,0.01),
    Vec2::new(-0.02,0.02),
    Vec2::new(0.01,0.01),
    Vec2::new(0.02,0.02),
    Vec2::new(0.01,0.01),
    Vec2::new(-0.01,0.01),
    Vec2::new(0.01,0.01),
    Vec2::new(0.0,0.02),
    Vec2::new(-0.01,0.01),
    Vec2::new(0.0,0.02),
];
const BULLET_VECTOR:&[Vec2]=&[
    Vec2::new(0.0,-0.005),
    Vec2::new(0.005,0.005),
    Vec2::new(0.0,-0.005),
    Vec2::new(-0.005,0.005),
];


struct Buffer(pub Buffer2d<u32>);
impl Deref for Buffer {
    type Target=Buffer2d<u32>;
    fn deref(&self)->&Self::Target {&self.0}
}
impl DerefMut for Buffer {
    fn deref_mut(&mut self)->&mut Self::Target {&mut self.0}
}
impl Dimensions for Buffer {
    fn bounding_box(&self)->Rectangle {
        Rectangle::new(
            Point::zero(),
            Size::new(
                self.0.size()[0] as u32,
                self.0.size()[1] as u32,
            ),
        )
    }
}
impl DrawTarget for Buffer {
    type Color=BinaryColor;
    type Error=();
    fn draw_iter<I:IntoIterator<Item=Pixel<Self::Color>>>(&mut self,pixels:I)->Result<(),()> {
        for pixel in pixels {
            let color=if pixel.1==BinaryColor::On {
                u32::from_le_bytes([200,200,200,0])   // light grey
            } else {
                0x000a0a99
            };
            let x=pixel.0.x as usize;
            let y=pixel.0.y as usize;
            if x<self.0.size()[0]&&y<self.0.size()[1] {
                unsafe {
                    self.0.set([x,y],color);
                }
            }
        }
        return Ok(());
    }
}
struct KeyMap {
    forward:Key,
    backward:Key,
    left:Key,
    right:Key,
    fire:Key,
}
struct Buttons {
    forward:bool,
    backward:bool,
    left:bool,
    right:bool,
    fire:bool,
}
struct Asteroid {
    pos:Vec2,
    vel:Vec2,
    radius:f32,
    model_index:usize,
}
struct Bullet {
    pos:Vec2,
    vel:Vec2,
    dir:f32,
}
struct Player {
    pos:Vec2,
    vel:Vec2,
    dir:f32,
    last_shoot:Instant,
}
struct Game {
    asteroids:Vec<Asteroid>,
    bullets:Vec<Bullet>,
    player:Player,
    game_over:bool,
    score:u64,
    size:Vec2,
    asteroid_vectors:Vec<Vec<Vec2>>,
    asteroid_vector_range:Range<usize>,
    rng:ThreadRng,
    _stream:OutputStream,
    stream_handle:OutputStreamHandle,
    sinks:Vec<Sink>,
    game_over_sink:Sink,
    collision_sound:Buffered<Decoder<BufReader<File>>>,
    shoot_sound:Buffered<Decoder<BufReader<File>>>,
    game_over_sound:Buffered<Decoder<BufReader<File>>>,
}
impl Pipeline for Game {
    type Vertex=Vec2;
    type VsOut=();
    type Pixel=u32;
    fn vert(&self,pos:&Self::Vertex)->([f32;4],Self::VsOut) {
        return ([pos.x,pos.y,0.0,1.0],());
    }
    fn frag(&self,_:&Self::VsOut)->Self::Pixel {
        u32::from_le_bytes([200,200,200,255])   // light grey
    }
}
impl Game {
    fn new(size:[f32;2])->Game {
        let collision_sound=Decoder::new_wav(BufReader::new(File::open("assets/sounds/asteroid_collision.wav").unwrap())).unwrap().buffered();
        let shoot_sound=Decoder::new_wav(BufReader::new(File::open("assets/sounds/shoot.wav").unwrap())).unwrap().buffered();
        let game_over_sound=Decoder::new_wav(BufReader::new(File::open("assets/sounds/game_over.wav").unwrap())).unwrap().buffered();
        let (stream,stream_handle)=OutputStream::try_default().unwrap();
        let sinks=vec![Sink::try_new(&stream_handle).unwrap()];
        let game_over_sink=Sink::try_new(&stream_handle).unwrap();
        let mut asteroid_vectors=Vec::new();
        for file in read_dir("assets/asteroids").unwrap() {
            let file=file.unwrap();
            let string=read_to_string(file.path()).unwrap();
            if let Some(vector)=svg_to_vector(&string) {
                asteroid_vectors.push(vector);
            }
        }
        let mut rng=thread_rng();
        let mut asteroid=Asteroid {
            pos:Vec2::new(rng.gen_range(0.0..1000.0),rng.gen_range(0.0..1000.0)),
            vel:Vec2::new(rng.gen_range(50.0..200.0),rng.gen_range(50.0..200.0)),
            radius:rng.gen_range(NEW_ASTEROID_MIN_RADIUS..ASTEROID_MAX_RADIUS),
            model_index:rng.gen_range(0..asteroid_vectors.len()),
        };
        while asteroid.pos.distance(Vec2::new(500.0,500.0))<asteroid.radius+PLAYER_RADIUS+50.0 {
            asteroid.pos=Vec2::new(rng.gen_range(0.0..1000.0),rng.gen_range(0.0..1000.0));
        }
        Game {
            _stream:stream,
            collision_sound,
            shoot_sound,
            game_over_sound,
            stream_handle,
            game_over_sink,
            sinks,
            asteroid_vector_range:0..asteroid_vectors.len(),
            asteroids:vec![asteroid],
            bullets:Vec::new(),
            player:Player {
                dir:0.0,
                pos:Vec2::new(500.0,500.0),
                vel:Vec2::zero(),
                last_shoot:Instant::now(),
            },
            game_over:false,
            score:0,
            size:Vec2::new(size[0],size[1]),
            asteroid_vectors,
            rng,
        }
    }
    fn reset(&mut self) {
        let mut asteroid=Asteroid {
            pos:Vec2::new(self.rng.gen_range(0.0..1000.0),self.rng.gen_range(0.0..1000.0)),
            vel:Vec2::new(self.rng.gen_range(50.0..200.0),self.rng.gen_range(50.0..200.0)),
            radius:self.rng.gen_range(NEW_ASTEROID_MIN_RADIUS..ASTEROID_MAX_RADIUS),
            model_index:self.rng.gen_range(self.asteroid_vector_range.clone()),
        };
        while asteroid.pos.distance(self.player.pos)<asteroid.radius+PLAYER_RADIUS+50.0 {
            asteroid.pos=Vec2::new(self.rng.gen_range(0.0..1000.0),self.rng.gen_range(0.0..1000.0));
        }
        self.asteroids=vec![asteroid];
        self.bullets=Vec::new();
        self.player=Player {
            dir:0.0,
            pos:Vec2::new(500.0,500.0),
            vel:Vec2::zero(),
            last_shoot:Instant::now(),
        };
        self.game_over=false;
        self.score=0;
    }
    fn is_game_over(&self)->bool {self.game_over}
    /// Processes the frame update and returns true while the game is running, and false if the
    /// player gets hit
    fn tick(&mut self,buttons:Buttons,delta:f32)->bool {
        // short-circuit and process nothing if we are in the game over state
        if self.game_over {return false}
        for a in self.asteroids.iter_mut() {
            a.pos+=a.vel*delta;
            if a.pos.x>=self.size.x {
                a.pos.x=0.0;
            } else if a.pos.x<0.0 {
                a.pos.x=self.size.x;
            }
            if a.pos.y>=self.size.y {
                a.pos.y=0.0;
            } else if a.pos.y<0.0 {
                a.pos.y=self.size.y;
            }
        }
        for b in self.bullets.iter_mut() {
            b.pos+=b.vel*delta;
            if b.pos.x>=self.size.x {
                b.pos.x=0.0;
            } else if b.pos.x<0.0 {
                b.pos.x=self.size.x;
            }
            if b.pos.y>=self.size.y {
                b.pos.y=0.0;
            } else if b.pos.y<0.0 {
                b.pos.y=self.size.y;
            }
        }
        self.player.pos+=self.player.vel*delta;
        if buttons.left {
            self.player.dir+=PLAYER_ROTATE_VEL.to_radians();
        }
        if buttons.right {
            self.player.dir-=PLAYER_ROTATE_VEL.to_radians();
        }
        if buttons.forward {
            self.player.vel-=Vec2::new(0.0,PLAYER_ACCEL*delta).rotated_z(self.player.dir);
        }
        if buttons.backward {
            self.player.vel-=Vec2::new(0.0,-PLAYER_ACCEL*delta).rotated_z(self.player.dir);
        }
        if self.player.pos.x>=self.size.x {
            self.player.pos.x=0.0;
        } else if self.player.pos.x<0.0 {
            self.player.pos.x=self.size.x;
        }
        if self.player.pos.y>=self.size.y {
            self.player.pos.y=0.0;
        } else if self.player.pos.y<0.0 {
            self.player.pos.y=self.size.y;
        }
        if buttons.fire {
            if self.player.last_shoot.elapsed()>=BULLET_DELAY {
                let mut played=false;
                for sink in self.sinks.iter_mut() {
                    if sink.empty() {
                        sink.append(self.shoot_sound.clone());
                        played=true;
                        break;
                    }
                }
                if !played {
                    let sink=Sink::try_new(&self.stream_handle).unwrap();
                    sink.append(self.shoot_sound.clone());
                    self.sinks.push(sink);
                }
                self.player.last_shoot=Instant::now();
                self.bullets.push(Bullet {
                    pos:self.player.pos,
                    vel:self.player.vel-Vec2::new(0.0,BULLET_VEL).rotated_z(self.player.dir),
                    dir:self.player.dir,
                });
            }
        }
        let mut new_asteroids=Vec::new();
        let mut asteroid_count=self.asteroids.len();
        self.asteroids.retain(|asteroid|{
            // Collide asteroid-bullet then delete the asteroid and bullet if they collide
            let mut hit=false;
            let mut idx=0;
            for (i,bullet) in self.bullets.iter().enumerate() {
                let dist_sq=asteroid.pos.distance_squared(bullet.pos);
                if dist_sq<=(asteroid.radius*asteroid.radius) {
                    hit=true;
                    idx=i;
                    break;
                }
            }
            // Collide asteroid-player then set game over
            if !self.game_over&&!DISABLE_GAME_OVER {
                let player_dist_sq=asteroid.pos.distance_squared(self.player.pos);
                if player_dist_sq<=(asteroid.radius+PLAYER_RADIUS).powi(2) {
                    self.game_over=true;
                    self.game_over_sink.append(self.game_over_sound.clone());
                }
            }
            // Remove the bullet if it was a hit
            if hit {
                let mut played=false;
                for sink in self.sinks.iter_mut() {
                    if sink.empty() {
                        sink.append(self.collision_sound.clone());
                        played=true;
                        break;
                    }
                }
                if !played {
                    let sink=Sink::try_new(&self.stream_handle).unwrap();
                    sink.append(self.collision_sound.clone());
                    self.sinks.push(sink);
                }
                asteroid_count-=1;
                self.bullets.remove(idx);
                self.score+=1;
                if asteroid_count<MAX_ASTEROIDS {
                    if asteroid.radius>ASTEROID_MIN_RADIUS {
                        let amt=self.rng.gen_range(2..=4);
                        let radius=asteroid.radius/(amt as f32);
                        asteroid_count+=amt;
                        for _ in 0..amt {
                            new_asteroids.push(Asteroid {
                                pos:asteroid.pos+Vec2::new(self.rng.gen_range(0.0..radius),self.rng.gen_range(0.0..radius)),
                                vel:asteroid.vel+Vec2::new(self.rng.gen_range(10.0..75.0),self.rng.gen_range(10.0..75.0)),
                                model_index:self.rng.gen_range(self.asteroid_vector_range.clone()),
                                radius,
                            });
                        }
                    } else {
                        let mut asteroid=Asteroid {
                            pos:Vec2::new(self.rng.gen_range(0.0..1000.0),self.rng.gen_range(0.0..1000.0)),
                            vel:Vec2::new(self.rng.gen_range(50.0..200.0),self.rng.gen_range(50.0..200.0)),
                            radius:self.rng.gen_range(NEW_ASTEROID_MIN_RADIUS..ASTEROID_MAX_RADIUS),
                            model_index:self.rng.gen_range(self.asteroid_vector_range.clone()),
                        };
                        while asteroid.pos.distance(self.player.pos)<asteroid.radius+PLAYER_RADIUS+50.0 {
                            asteroid.pos=Vec2::new(self.rng.gen_range(0.0..1000.0),self.rng.gen_range(0.0..1000.0));
                        }
                        new_asteroids.push(asteroid);
                    }
                }
            }
            !hit
        });
        self.asteroids.append(&mut new_asteroids);
        // Collide asteroid-asteroid in a non-physical way that cheats
        let mut collisions=HashMap::new();
        for (a,asteroid_a) in self.asteroids.iter().enumerate() {
            for (b,asteroid_b) in self.asteroids.iter().enumerate() {
                if a!=b {
                    let dist=asteroid_a.pos.distance(asteroid_b.pos);
                    if dist<=(asteroid_a.radius+asteroid_b.radius) {
                        let angle=asteroid_a.vel.angle_between(asteroid_b.vel);
                        let move_vec=(asteroid_a.pos-asteroid_b.pos).normalized();
                        let move_amt=dist-(asteroid_a.radius+asteroid_b.radius);
                        let sq_sum=asteroid_a.radius.powi(2)+asteroid_b.radius.powi(2);
                        let ratio_a=asteroid_a.radius.powi(2)/sq_sum;
                        let ratio_b=asteroid_b.radius.powi(2)/sq_sum;
                        collisions.insert(a,(-(angle*ratio_b),-move_vec*move_amt));
                        collisions.insert(b,((angle*ratio_a),move_vec*move_amt));
                    }
                }
            }
        }
        for (i,(angle,move_amt)) in collisions {
            self.asteroids[i].vel.rotate_z(angle);
            self.asteroids[i].pos+=move_amt;
        }
        !self.game_over
    }
    fn render(&mut self,buffer:&mut Buffer2d<u32>) {
        let mut vertices=Vec::new();
        for asteroid in self.asteroids.iter() {
            let pos=asteroid.pos;
            let pos=(pos/(self.size/2.0))-1.0;
            for vtx in self.asteroid_vectors[asteroid.model_index].iter() {
                let vtx=(vtx*(asteroid.radius/4.0))/10000.0;
                let res=vtx+pos;
                vertices.push(res);
            }
        }
        for bullet in self.bullets.iter() {
            let pos=bullet.pos;
            let pos=(pos/(self.size/2.0))-1.0;
            for vtx in BULLET_VECTOR {
                let res=(vtx.rotated_z(bullet.dir))+pos;
                vertices.push(res);
            }
        }
        let pos=self.player.pos;
        let pos=(pos/(self.size/2.0))-1.0;
        for vtx in PLAYER_VECTOR {
            let res=(vtx.rotated_z(self.player.dir))+pos;
            vertices.push(res);
        }
        assert!(vertices.len()%2==0);
        self.draw::<Lines<(f32,)>,_>(
            &vertices,
            buffer,
            None,
        );
    }
}


fn main() {
    let mut buffer=Buffer(Buffer2d::new([1000,1000],0u32));
    let mut window=Window::new("Asteroids",1000,1000,Default::default()).unwrap();
    window.limit_update_rate(Some(Duration::from_secs_f32(1.0/(FPS as f32))));
    let mut last_frame=Instant::now();
    let mut game=Game::new([1000.0,1000.0]);
    Text::with_text_style("WASD to move\nF to fire",Point::new(500,500),TextStyle::new(&FONT_10x20, BinaryColor::On),EgTextStyle::with_alignment(Alignment::Center))
        .draw(&mut buffer).unwrap();
    window.update_with_buffer(&buffer.0.as_ref(),1000,1000).unwrap();
    std::thread::sleep(Duration::from_secs(2));
    while window.is_open() {
        buffer.0.clear(0);
        let elapsed=last_frame.elapsed();
        last_frame=Instant::now();
        game.tick(
            Buttons {
                forward:window.is_key_down(KEYMAP.forward),
                backward:window.is_key_down(KEYMAP.backward),
                left:window.is_key_down(KEYMAP.left),
                right:window.is_key_down(KEYMAP.right),
                fire:window.is_key_down(KEYMAP.fire),
            },
            elapsed.as_secs_f32(),
        );
        game.render(&mut buffer.0);
        Text::new(&format!("Score: {}",game.score), Point::zero(), TextStyle::new(&FONT_10x20, BinaryColor::On))
            .draw(&mut buffer).unwrap();
        if game.is_game_over() {
            let style=PrimitiveStyleBuilder::new()
                .fill_color(BinaryColor::Off)
                .build();
            Rectangle::with_center(Point::new(500,520),Size::new(200,76))
                .into_styled(style)
                .draw(&mut buffer)
                .unwrap();
            /*
             * (400,482)    (600,482)
             *
             * (400,558)    (600,558)
             */
            Text::with_text_style("Game over!\nClick to restart",Point::new(500,500),TextStyle::new(&FONT_10x20, BinaryColor::On),EgTextStyle::with_alignment(Alignment::Center))
                .draw(&mut buffer).unwrap();
            if window.get_mouse_down(MouseButton::Left) {
                if let Some((x,y))=window.get_mouse_pos(MouseMode::Discard) {
                    if (x<600.0&&x>400.0)&&(y<558.0&&y>482.0) {   // if in the button
                        game.reset();
                    }
                }
            }
        }
        window.update_with_buffer(&buffer.0.as_ref(),1000,1000).unwrap();
    }
}
