use std::{f32::consts::PI, net::Ipv4Addr, path::PathBuf, str::FromStr, sync::{atomic::{AtomicI32, AtomicU8, AtomicUsize}, Arc, RwLock}, thread, time::{Duration, Instant}};

use cosmic_text::{Color, Metrics};
use crossbeam::channel::unbounded;
use task_derive::HordeTask;

use crate::{defaults::{default_frontends::minifb_frontend::MiniFBWindow, default_rendering::vectorinator::{meshes::{Mesh, MeshID, MeshInstance, MeshLOD, MeshLODS, MeshLODType, MeshTriangles, TrianglePoint}, textures::rgb_to_argb, Vectorinator}, default_ui::simple_ui::{SimpleUI, TextCentering, UIDimensions, UIElement, UIElementBackground, UIElementContent, UIElementID, UIEvent, UIUnit, UIUserAction, UIVector, UserEvent}}, horde::{frontend::{HordeWindowDimensions, SyncUnsafeHordeFramebuffer, WindowingHandler}, game_engine::{multiplayer::HordeMultiModeChoice, world::WorldHandler}, geometry::{rotation::Orientation, vec3d::Vec3Df}, rendering::{camera::Camera, framebuffer::HordeColorFormat}, scheduler::{HordeScheduler, HordeTask, HordeTaskData, HordeTaskHandler, HordeTaskQueue, HordeTaskSequence, IndividualTask, SequencedTask}, sound::{ARWWaves, SoundRequest, WaveIdentification, WavePosition, WaveRequest, WaveSink, Waves, WavesHandler}}};

use super::{engine_derive_test::{TestEngineBase, TestWorld}, entity_derive_test::{CoolComponent, CoolEntityVec, NewCoolEntity}, single_player_engine_test::{SinglePEngine, SinglePEngineBase, SinglePWorld}};

pub fn lance_serveur() {
    let world = TestWorld { test: 1};
    let entity_vec = CoolEntityVec::new(1000);
    let vectorinator = Vectorinator::new(Arc::new(RwLock::new(SyncUnsafeHordeFramebuffer::new(HordeWindowDimensions::new(100, 100), HordeColorFormat::ARGB8888))));
    let engine = TestEngineBase::new(entity_vec, WorldHandler::new(world), Arc::new(vectorinator.clone()), HordeMultiModeChoice::Server { adress: (Ipv4Addr::new(127, 0, 0, 1), 5678), max_players: 100, tick_tolerance: 10,tickrate:30 }, 1);
    let handler = TestServerTaskTaskHandler::new(engine);
    let queue = HordeTaskQueue::new(vec![HordeTaskSequence::new(vec![
        SequencedTask::StartTask(TestServerTask::Main),
        SequencedTask::WaitFor(TestServerTask::Main),
        SequencedTask::StartTask(TestServerTask::ApplyEvents),
        SequencedTask::WaitFor(TestServerTask::ApplyEvents),
        SequencedTask::StartTask(TestServerTask::AfterMain),
        SequencedTask::WaitFor(TestServerTask::AfterMain),
        SequencedTask::StartTask(TestServerTask::ApplyEvents),
        SequencedTask::WaitFor(TestServerTask::ApplyEvents),
        SequencedTask::StartTask(TestServerTask::SendMustSync),
        SequencedTask::WaitFor(TestServerTask::SendMustSync),
        SequencedTask::StartTask(TestServerTask::MultiFirstPart),
        SequencedTask::WaitFor(TestServerTask::MultiFirstPart),
        SequencedTask::StartTask(TestServerTask::MultiSecondPart),
        SequencedTask::WaitFor(TestServerTask::MultiSecondPart),
        SequencedTask::StartTask(TestServerTask::MultiThirdPart),
        SequencedTask::WaitFor(TestServerTask::MultiThirdPart),
        SequencedTask::StartTask(TestServerTask::MultiFourthPart),
        SequencedTask::WaitFor(TestServerTask::MultiFourthPart),
        ]
    )], Vec::new());
    let mut scheduler = HordeScheduler::new(queue.clone(), handler, 3);
    println!("SERVER LOOP STARTING");
    loop {
        let start = Instant::now();
        scheduler.initialise(queue.clone());
        scheduler.tick();
        let duration = start.elapsed().as_secs_f64();
        if duration < 1.0/30.0 {
            thread::sleep(Duration::from_secs_f64(1.0/30.0 - duration));
            //println!("Server TPS : {} per second", 1.0/duration);
        }
    }
}

#[derive(Clone)]
pub struct SingleExtraData {
    pub tick:Arc<AtomicUsize>,
    pub waves_handler:WavesHandler<SinglePEngine>
}

pub fn singleplayer_test() {
    let world = SinglePWorld { test: 1};
    let entity_vec = CoolEntityVec::new(1000);
    {
        entity_vec.get_write().new_ent(NewCoolEntity::new(CoolComponent {pos:Vec3Df::zero()}, false, None));
    }
    
    let windowing = WindowingHandler::new::<MiniFBWindow>(HordeWindowDimensions::new(1280, 720), HordeColorFormat::ARGB8888);
    let framebuf = windowing.get_outside_framebuf();
    let vectorinator = Vectorinator::new(framebuf.clone());
    let (waves, waves_handler, stream) = Waves::new(Vec::new(), 10);
    let engine = SinglePEngineBase::new(entity_vec, WorldHandler::new(world), Arc::new(vectorinator.clone()), SingleExtraData { tick: Arc::new(AtomicUsize::new(0)), waves_handler:waves_handler.clone()});
    waves_handler.send_gec(engine.clone());
    let mouse = windowing.get_mouse_state();
    let (mut simpleui, user_events) = SimpleUI::new(20, 20, framebuf.clone(), mouse, unbounded().1);
    let test_ui_element = UIElement::new(
        UIVector::new(UIUnit::ParentWidthProportion(0.10), UIUnit::ParentHeightProportion(0.10)),
        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.4), UIUnit::ParentHeightProportion(0.3))),
        UIVector::new(UIUnit::RelativeToParentOrigin(10), UIUnit::RelativeToParentOrigin(10)),
        None,
        "TestWidget".to_string()
    )
    .with_content_background(UIElementBackground::Color(rgb_to_argb((50, 50, 50))))
    .with_content_background(UIElementBackground::Color(rgb_to_argb((255, 0, 0))))
    .with_content_background(UIElementBackground::Color(rgb_to_argb((0, 0, 255))))
    .with_background(UIElementBackground::Color(rgb_to_argb((125, 125, 125))))
    .with_reaction((UIUserAction::Nothing, UIEvent::ChangeContentBackground(1)))
    .with_reaction((UIUserAction::Clicking, UIEvent::ChangeContentBackground(2)))
    .with_child(UIElementID::Name("TestWidgetChild".to_string()));
    simpleui.add_element(test_ui_element);
    let test_ui_child = UIElement::new(
        UIVector::new(UIUnit::ParentWidthProportion(0.10), UIUnit::ParentHeightProportion(0.10)),
        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(0.5))),
        UIVector::new(UIUnit::RelativeToParentOrigin(20), UIUnit::RelativeToParentOrigin(20)),
        Some(UIElementID::Name("TestWidget".to_string())),
        "TestWidgetChild".to_string()
    )
    .with_content_background(UIElementBackground::Color(rgb_to_argb((50, 50, 50))))
    .with_content_background(UIElementBackground::Color(rgb_to_argb((255, 0, 0))))
    .with_content_background(UIElementBackground::Color(rgb_to_argb((0, 0, 255))))
    .with_background(UIElementBackground::Color(rgb_to_argb((125, 125, 125))))
    .with_reaction((UIUserAction::Nothing, UIEvent::ChangeContentBackground(1)))
    .with_reaction((UIUserAction::Clicking, UIEvent::ChangeContentBackground(2)))
    .with_reaction((UIUserAction::Nothing, UIEvent::ChangeContent(1)))
    .with_reaction((UIUserAction::Clicking, UIEvent::ChangeContent(2)))
    .with_reaction((UIUserAction::Clicking, UIEvent::User(TestUserEvent::ClickedCoolButton)))
    .with_content(UIElementContent::Text { text: "cosmic-text my beloved".to_string(), font: "rien".to_string(), metrics:Metrics::new(20.0, 25.0), color:Color::rgb(255, 255, 255), centering:TextCentering::Neither })
    .with_content(UIElementContent::Text { text: "you're hovering...".to_string(), font: "rien".to_string(), metrics:Metrics::new(20.0, 25.0), color:Color::rgb(0, 255, 255), centering:TextCentering::Neither })
    .with_content(UIElementContent::Text { text: "YOU'RE CLICKING".to_string(), font: "rien".to_string(), metrics:Metrics::new(20.0, 25.0), color:Color::rgb(255, 255, 0), centering:TextCentering::Neither });
    simpleui.add_element(test_ui_child);
    simpleui.add_image(PathBuf::from("textures/coolfrog.jpg"), Some("FROG_TEST".to_string()));
    let test_ui_image = UIElement::new(
        UIVector::new(UIUnit::ParentWidthProportion(0.5), UIUnit::ParentHeightProportion(0.5)),
        UIDimensions::Decided(UIVector::new(UIUnit::ParentWidthProportion(0.4), UIUnit::ParentHeightProportion(0.4))),
        UIVector::new(UIUnit::RelativeToParentOrigin(20), UIUnit::RelativeToParentOrigin(20)),
        None,
        "Cool Image".to_string()
    )
    .with_background(UIElementBackground::Image("FROG_TEST".to_string()))
    .change_visibility(false);
    simpleui.add_element(test_ui_image);

    let handler = TestSinglePlayerTaskTaskHandler::new(engine, windowing, vectorinator.clone(), simpleui.clone(), waves);
    
    let queue = HordeTaskQueue::new(vec![HordeTaskSequence::new(vec![

        SequencedTask::StartTask(TestSinglePlayerTask::PrepareRendering),
        SequencedTask::WaitFor(TestSinglePlayerTask::PrepareRendering),
        SequencedTask::StartSequence(1),
        SequencedTask::StartTask(TestSinglePlayerTask::UpdateSoundPositions),
        SequencedTask::StartTask(TestSinglePlayerTask::Main),
        SequencedTask::WaitFor(TestSinglePlayerTask::Main),
        SequencedTask::WaitFor(TestSinglePlayerTask::UpdateSoundPositions),
        SequencedTask::StartTask(TestSinglePlayerTask::UpdateSoundEverythingElse),
        SequencedTask::StartTask(TestSinglePlayerTask::ApplyEvents),
        SequencedTask::WaitFor(TestSinglePlayerTask::ApplyEvents),
        SequencedTask::StartTask(TestSinglePlayerTask::AfterMain),
        SequencedTask::WaitFor(TestSinglePlayerTask::AfterMain),
        SequencedTask::StartTask(TestSinglePlayerTask::ApplyEvents),
        SequencedTask::WaitFor(TestSinglePlayerTask::ApplyEvents),
        SequencedTask::WaitFor(TestSinglePlayerTask::UpdateSoundEverythingElse),
        ]
    ),
    HordeTaskSequence::new(vec![
        SequencedTask::StartTask(TestSinglePlayerTask::DoAllUIRead),
        SequencedTask::StartTask(TestSinglePlayerTask::DoEventsAndMouse),
        SequencedTask::StartTask(TestSinglePlayerTask::ResetCounters),
        SequencedTask::WaitFor(TestSinglePlayerTask::ResetCounters),
        SequencedTask::StartTask(TestSinglePlayerTask::RenderEverything),
        SequencedTask::WaitFor(TestSinglePlayerTask::RenderEverything),
        SequencedTask::WaitFor(TestSinglePlayerTask::DoAllUIRead),
        SequencedTask::StartTask(TestSinglePlayerTask::DoAllUIWrite),
        SequencedTask::WaitFor(TestSinglePlayerTask::DoAllUIWrite),
        SequencedTask::StartTask(TestSinglePlayerTask::ClearZbuf),
        SequencedTask::WaitFor(TestSinglePlayerTask::DoEventsAndMouse),
        SequencedTask::StartTask(TestSinglePlayerTask::SendFramebuf),
        SequencedTask::WaitFor(TestSinglePlayerTask::SendFramebuf),
        SequencedTask::StartTask(TestSinglePlayerTask::ClearFramebuf),
        SequencedTask::StartTask(TestSinglePlayerTask::WaitForPresent),
        SequencedTask::WaitFor(TestSinglePlayerTask::WaitForPresent),
        SequencedTask::WaitFor(TestSinglePlayerTask::ClearZbuf),
        SequencedTask::WaitFor(TestSinglePlayerTask::ClearFramebuf),
        SequencedTask::StartTask(TestSinglePlayerTask::TickAllSets),
        SequencedTask::WaitFor(TestSinglePlayerTask::TickAllSets),
        ]
    )], Vec::new());
    {
        let mut writer = vectorinator.get_write();
        writer.textures.add_set_with_many_textures(
            "Testing_Texture".to_string(),
            vec![
                (
                    "terre_herbe.png".to_string(),
                    1,
                    None
                )
            ]
        );
        let mut triangles = MeshTriangles::with_capacity(128);
        triangles.add_triangle(
            TrianglePoint::new(0, 0.0, 0.0, 255, 255, 255),
            TrianglePoint::new(1, 0.0, 1.0, 255, 255, 255),
            TrianglePoint::new(2, 1.0, 1.0, 255, 255, 255),
            
            0, 0
        );
        triangles.add_triangle(
            TrianglePoint::new(0, 0.0, 0.0, 255, 255, 255),
            TrianglePoint::new(2, 1.0, 1.0, 255, 255, 255),
            TrianglePoint::new(3, 1.0, 0.0, 255, 255, 255),
            
            0, 0
        );
        
        let test_mesh_id = writer.meshes.add_mesh(Mesh::new(
        MeshLODS::new(vec![
            MeshLODType::Mesh(
                Arc::new(MeshLOD::new(
                    vec![0.0, 1.0, 1.0, 0.0],
                    vec![0.0, 0.0, 1.0, 1.0],
                    vec![0.0, 0.0, 0.0, 0.0], 
                    triangles
                ))
            )
        ]),
        String::from("Test Cube Mesh"),
        2.0
        ));
        // println!("TEST ID {}", test_mesh_id);
        for x in -100..100 {
            for y in -100..100 {
                for z in -15..15 {
                    if !(x == 0 && y == 0) {
                        writer.meshes.add_instance(MeshInstance::new(Vec3Df::new((x * 2) as f32, (y * 2) as f32, (z * 2) as f32), Orientation::zero(), MeshID::Referenced(test_mesh_id), true, false, false), 0);
                    }
                }
                
            }
        }
        *writer.camera = Camera::new(Vec3Df::new(0.0, 0.0, 20.0), Orientation::new(0.0, PI, 0.0));
    }
    let mut scheduler = HordeScheduler::new(queue.clone(), handler, 16);
    let mut clicked = false;
    for i in 0..255 {
        //println!("{i}");
        let mut start = Instant::now();
        {
            let mut writer = vectorinator.get_write();
            *writer.camera = Camera::new(Vec3Df::new((i as f32 / 100.0) * 2.5 + 0.1, 0.1, -50.0), Orientation::new((i as f32 / 255.0) * 0.1 * PI, 0.0, PI/3.0));//(i as f32 / 500.0) * PI/2.0));
            /*thread::sleep(Duration::from_millis(10));*/
        }
        match user_events.try_recv() {
            Ok(evt) => match evt {
                TestUserEvent::ClickedCoolButton => {
                    if !clicked {
                        simpleui.change_visibility_of(UIElementID::Name("Cool Image".to_string()), true);
                        waves_handler.request_sound(WaveRequest::Sound(SoundRequest::new(WaveIdentification::ByName("vine-boom.mp3".to_string()), WavePosition::InsideYourHead, WaveSink::FirstEmpty)));
                        clicked = true;
                    }
                },
                _ => ()
            }
            Err(_) => ()
        }
        scheduler.initialise(queue.clone());
        scheduler.tick();
        //println!("FPS : {}", 1.0/Instant::now().checked_duration_since(start).unwrap().as_secs_f64())
    }
    scheduler.end_threads();
}

pub fn client_test(name:String) {
    thread::sleep(Duration::from_secs(1));
    println!("CLIENT {} STARTING", name.clone());
    let world = TestWorld { test: 1};
    let entity_vec = CoolEntityVec::new(1000);
    let windowing = WindowingHandler::new::<MiniFBWindow>(HordeWindowDimensions::new(720, 480), HordeColorFormat::ARGB8888);
    let framebuf = windowing.get_outside_framebuf();
    let vectorinator = Vectorinator::new(framebuf.clone());
    let (cs, cr) = unbounded();
    let engine = TestEngineBase::new(entity_vec, WorldHandler::new(world), Arc::new(vectorinator.clone()), HordeMultiModeChoice::Client { adress: Some((Ipv4Addr::new(127, 0, 0, 1), 5678)), name:name.clone(), chat:cr }, 1);
    
    let handler = TestTaskTaskHandler::new(engine, windowing, vectorinator.clone());
    let queue = HordeTaskQueue::new(vec![HordeTaskSequence::new(vec![
        SequencedTask::StartSequence(1),
        SequencedTask::StartTask(TestTask::Main),
        SequencedTask::WaitFor(TestTask::Main),
        SequencedTask::StartTask(TestTask::ApplyEvents),
        SequencedTask::WaitFor(TestTask::ApplyEvents),
        SequencedTask::StartTask(TestTask::AfterMain),
        SequencedTask::WaitFor(TestTask::AfterMain),
        SequencedTask::StartTask(TestTask::ApplyEvents),
        SequencedTask::WaitFor(TestTask::ApplyEvents),
        SequencedTask::StartTask(TestTask::MultiFirstPart),
        SequencedTask::WaitFor(TestTask::MultiFirstPart),
        SequencedTask::StartTask(TestTask::MultiSecondPart),
        SequencedTask::WaitFor(TestTask::MultiSecondPart),
        ]
    ),
    HordeTaskSequence::new(vec![
        SequencedTask::StartTask(TestTask::ResetCounters),
        SequencedTask::WaitFor(TestTask::ResetCounters),
        SequencedTask::StartTask(TestTask::RenderEverything),
        SequencedTask::WaitFor(TestTask::RenderEverything),
        SequencedTask::StartTask(TestTask::ClearZbuf),
        SequencedTask::StartTask(TestTask::SendFramebuf),
        SequencedTask::WaitFor(TestTask::SendFramebuf),
        SequencedTask::StartTask(TestTask::ClearFramebuf),
        SequencedTask::StartTask(TestTask::WaitForPresent),
        SequencedTask::WaitFor(TestTask::WaitForPresent),
        SequencedTask::WaitFor(TestTask::ClearZbuf),
        SequencedTask::WaitFor(TestTask::ClearFramebuf),
        SequencedTask::StartTask(TestTask::TickAllSets),
        SequencedTask::WaitFor(TestTask::TickAllSets),
        ]
    )], Vec::new());
    {
        let mut writer = vectorinator.get_write();
        writer.textures.add_set_with_many_textures(
            "Testing_Texture".to_string(),
            vec![
                (
                    "terre_herbe.png".to_string(),
                    1,
                    None
                )
            ]
        );
        let mut triangles = MeshTriangles::with_capacity(128);
        triangles.add_triangle(
            TrianglePoint::new(0, 0.0, 0.0, 255, 255, 255),
            TrianglePoint::new(1, 0.0, 1.0, 255, 255, 255),
            TrianglePoint::new(2, 1.0, 1.0, 255, 255, 255),
            
            0, 0
        );
        triangles.add_triangle(
            TrianglePoint::new(0, 0.0, 0.0, 255, 255, 255),
            TrianglePoint::new(2, 1.0, 1.0, 255, 255, 255),
            TrianglePoint::new(3, 1.0, 0.0, 255, 255, 255),
            
            0, 0
        );
        
        let test_mesh_id = writer.meshes.add_mesh(Mesh::new(
        MeshLODS::new(vec![
            MeshLODType::Mesh(
                Arc::new(MeshLOD::new(
                    vec![0.0, 1.0, 1.0, 0.0],
                    vec![0.0, 0.0, 1.0, 1.0],
                    vec![0.0, 0.0, 0.0, 0.0], 
                    triangles
                ))
            )
            
        ]),
        String::from("Test Cube Mesh"),
        2.0
        ));
        //println!("{}", test_mesh_id);
        for x in -100..100 {
            for y in -100..100 {
                for z in -15..15 {
                    if !(x == 0 && y == 0) {
                        writer.meshes.add_instance(MeshInstance::new(Vec3Df::new((x * 2) as f32, (y * 2) as f32, (z * 2) as f32), Orientation::zero(), MeshID::Referenced(test_mesh_id), true, false, false), 0);
                    }
                }
                
            }
        }
        *writer.camera = Camera::new(Vec3Df::new(0.0, 0.0, 20.0), Orientation::new(0.0, PI, 0.0));
    }
    let mut scheduler = HordeScheduler::new(queue.clone(), handler, 16);
    for i in 0..2550 {
        if i == 25 {
            cs.send(format!("{} is sending you this message", name.clone()));
            println!("{} SENT MESSAGE", name);
        }
        //println!("{i}");
        let mut start = Instant::now();
        {
            let mut writer = vectorinator.get_write();
            *writer.camera = Camera::new(Vec3Df::new((i as f32 / 100.0) * 2.5 + 0.1, 0.1, -50.0), Orientation::new((i as f32 / 255.0) * 0.1 * PI, 0.0, PI/3.0));//(i as f32 / 500.0) * PI/2.0));
            /*thread::sleep(Duration::from_millis(10));*/
        }
        scheduler.initialise(queue.clone());
        scheduler.tick();
        //println!("FPS : {}", 1.0/Instant::now().checked_duration_since(start).unwrap().as_secs_f64())
    }
    scheduler.end_threads();
}

pub fn testst() {
    thread::spawn(|| {
        lance_serveur();
    });
    for i in 0..2 {
        thread::spawn(move || {
            client_test(format!("PSEUDO_TEST_{}", i));
        });
    }
    client_test(format!("PSEUDO_TEST_{}", 3));
    
}



#[derive(Clone, PartialEq, Hash, Eq, Debug, HordeTask)]
pub enum TestTask {
    #[uses_type = "TestEngineBase"]
    #[max_threads = 1]
    #[type_task_id = 0]
    ApplyEvents,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 3]
    #[type_task_id = 1]
    Main,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 3]
    #[type_task_id = 2]
    AfterMain,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 1]
    #[type_task_id = 3]
    PrepareRendering,

    #[uses_type = "WindowingHandler"]
    #[max_threads = 1]
    #[type_task_id = 0]
    SendFramebuf,

    #[uses_type = "WindowingHandler"]
    #[max_threads = 1]
    #[type_task_id = 1]
    WaitForPresent,

    #[uses_type = "Vectorinator"]
    #[max_threads = 16]
    #[type_task_id = 0]
    RenderEverything,

    #[uses_type = "Vectorinator"]
    #[max_threads = 1]
    #[type_task_id = 1]
    TickAllSets,

    #[uses_type = "Vectorinator"]
    #[max_threads = 1]
    #[type_task_id = 2]
    ResetCounters,

    #[uses_type = "Vectorinator"]
    #[max_threads = 1]
    #[type_task_id = 3]
    ClearFramebuf,

    #[uses_type = "Vectorinator"]
    #[max_threads = 1]
    #[type_task_id = 4]
    ClearZbuf,
    
    #[uses_type = "TestEngineBase"]
    #[max_threads = 1]
    #[type_task_id = 30]
    SendMustSync,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 1]
    #[type_task_id = 20]
    MultiFirstPart,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 1]
    #[type_task_id = 21]
    MultiSecondPart,
}

#[derive(Clone, PartialEq, Hash, Eq, Debug, HordeTask)]
pub enum TestServerTask {
    #[uses_type = "TestEngineBase"]
    #[max_threads = 1]
    #[type_task_id = 0]
    ApplyEvents,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 3]
    #[type_task_id = 1]
    Main,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 3]
    #[type_task_id = 2]
    AfterMain,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 1]
    #[type_task_id = 30]
    SendMustSync,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 1]
    #[type_task_id = 10]
    MultiFirstPart,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 4]
    #[type_task_id = 11]
    MultiSecondPart,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 1]
    #[type_task_id = 12]
    MultiThirdPart,

    #[uses_type = "TestEngineBase"]
    #[max_threads = 4]
    #[type_task_id = 13]
    MultiFourthPart,
}


#[derive(Clone, PartialEq, Hash, Eq, Debug, HordeTask)]
pub enum TestSinglePlayerTask {
    #[uses_type = "SinglePEngineBase"]
    #[max_threads = 1]
    #[type_task_id = 0]
    ApplyEvents,

    #[uses_type = "SinglePEngineBase"]
    #[max_threads = 3]
    #[type_task_id = 1]
    Main,

    #[uses_type = "SinglePEngineBase"]
    #[max_threads = 3]
    #[type_task_id = 2]
    AfterMain,

    #[uses_type = "SinglePEngineBase"]
    #[max_threads = 1]
    #[type_task_id = 3]
    PrepareRendering,

    #[uses_type = "WindowingHandler"]
    #[max_threads = 1]
    #[type_task_id = 0]
    SendFramebuf,

    #[uses_type = "WindowingHandler"]
    #[max_threads = 1]
    #[type_task_id = 1]
    WaitForPresent,

    #[uses_type = "WindowingHandler"]
    #[max_threads = 1]
    #[type_task_id = 2]
    DoEventsAndMouse,

    #[uses_type = "Vectorinator"]
    #[max_threads = 16]
    #[type_task_id = 0]
    RenderEverything,

    #[uses_type = "Vectorinator"]
    #[max_threads = 1]
    #[type_task_id = 1]
    TickAllSets,

    #[uses_type = "Vectorinator"]
    #[max_threads = 1]
    #[type_task_id = 2]
    ResetCounters,

    #[uses_type = "Vectorinator"]
    #[max_threads = 1]
    #[type_task_id = 3]
    ClearFramebuf,

    #[uses_type = "Vectorinator"]
    #[max_threads = 1]
    #[type_task_id = 4]
    ClearZbuf,

    #[uses_type = "SimpleUI"]
    #[uses_generic = "TestUserEvent"]
    #[max_threads = 1]
    #[type_task_id = 0]
    DoAllUIRead,

    #[uses_type = "SimpleUI"]
    #[uses_generic = "TestUserEvent"]
    #[max_threads = 1]
    #[type_task_id = 1]
    DoAllUIWrite,

    #[uses_type = "ARWWaves"]
    #[uses_generic = "SinglePEngine"]
    #[max_threads = 1]
    #[type_task_id = 0]
    UpdateSoundPositions,

    #[uses_type = "ARWWaves"]
    #[uses_generic = "SinglePEngine"]
    #[max_threads = 1]
    #[type_task_id = 1]
    UpdateSoundEverythingElse,
    
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub enum TestUserEvent {
    ClickedCoolButton,
    ClickedBadButton
}

impl UserEvent for TestUserEvent {

}