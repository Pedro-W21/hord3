use std::{collections::HashMap, fmt::{Debug, Display}, hash::Hash, thread};

use crossbeam::channel::{Receiver, Sender, unbounded};


pub trait HordeTaskHandler:Clone {

}
pub trait HordeTask: Hash + Eq + PartialEq  + Clone + Send + Sync + Debug {
    type HTH:HordeTaskHandler;
    type HTD:HordeTaskData<Self>;
    fn max_threads(&self) -> usize;
    fn data_from_handler(handler:&Self::HTH) -> Self::HTD;
}

pub trait HordeTaskData<HT:HordeTask>:Send + Sync {
    fn do_task(&mut self, task:HT, thread_number:usize, number_of_threads:usize);
}

#[derive(Clone)]
pub struct HordeTaskQueue<HT:HordeTask> {
    tasks:Vec<HordeTaskSequence<HT>>,
    must_be_finished_before_next:Vec<HT>
}

impl<HT:HordeTask> HordeTaskQueue<HT> {
    pub fn new(tasks:Vec<HordeTaskSequence<HT>>, must_be_finished_before_next:Vec<HT>) -> Self {
        Self { tasks, must_be_finished_before_next }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub enum SequenceState {
    NotStarted,
    Started,
    Finished,
}
#[derive(Clone)]
pub struct HordeTaskSequence<HT:HordeTask> {
    state:SequenceState,
    seq:Vec<SequencedTask<HT>>,
    position:usize,
}

impl<HT:HordeTask> HordeTaskSequence<HT> {
    pub fn new(seq:Vec<SequencedTask<HT>>) -> Self {
        Self { state: SequenceState::NotStarted, seq, position:0 }
    }
    fn current_task(&self) -> SequencedTask<HT> {
        self.seq[self.position].clone()
    }
    fn start_sequence(&mut self) {
        self.state = SequenceState::Started;
    }
    fn get_state(&self) -> SequenceState {
        self.state.clone()
    }
    fn advance_sequence(&mut self) {
        self.position += 1;
        if self.position >= self.seq.len() {
            self.state = SequenceState::Finished
        }
    }
}

#[derive(Clone)]
pub enum SequencedTask<HT:HordeTask> {
    WaitFor(HT),
    StartSequence(usize),
    StartTask(HT)
}

pub struct HordeScheduler<HT:HordeTask> {
    handler:HT::HTH,
    current_tasks:HordeTaskQueue<HT>,
    task_counter:HashMap<HT, usize>,
    send:Sender<SchedulerTask<HT>>,
    rcv:Receiver<HT>,
    number_of_threads:usize,
    idle_threads:usize,
    tasks_in_flight:usize
}

pub enum SchedulerTask<HT:HordeTask> {
    Task{tsk:HT, thread_number:usize, number_of_threads_on_task:usize},
    Stop,
}

impl<HT:HordeTask + 'static> HordeScheduler<HT> {
    pub fn new(initial_queue:HordeTaskQueue<HT>, handler:HT::HTH, number_of_threads:usize) -> Self {
        let task_counter = HashMap::new();
        let (send_task, receive_task) = unbounded();
        let (send_stop, recveive_stop) = unbounded();
        let mut out = Self {handler, task_counter, current_tasks:initial_queue, send:send_task, rcv:recveive_stop, number_of_threads, idle_threads:number_of_threads, tasks_in_flight:0};
        for i in 0..number_of_threads {
            let send_clone = send_stop.clone();
            let rcv_clone = receive_task.clone();
            let data = HT::data_from_handler(&out.handler);
            thread::spawn(move || {
                task_thread(rcv_clone, send_clone, data);
            });
        }
        out
    }
    pub fn initialise(&mut self, new_queue:HordeTaskQueue<HT>) {
        self.current_tasks = new_queue;
        self.current_tasks.tasks[0].start_sequence();
    }

    pub fn tick(&mut self) {
        let mut finished = false;
        while !finished {
            if !self.advance_all_sequences() && !self.receive_stop() {
                finished = true;
            }
        }
    }
    pub fn receive_stop(&mut self) -> bool {
        let mut anything_to_receive = false;
        for task in &self.current_tasks.must_be_finished_before_next {
            match self.task_counter.get(task) {
                Some(counter) => if *counter > 0 {anything_to_receive = true; break},
                None => (),
            }
        }
        if anything_to_receive {
            match self.rcv.recv() {
                Ok(stop) => match self.task_counter.get_mut(&stop) {
                    Some(counter) => {
                        *counter -= 1;
                        if self.current_tasks.must_be_finished_before_next.contains(&stop) && *counter == 0 {
                            self.current_tasks.must_be_finished_before_next.retain(|tsk| {*tsk != stop});
                            //dbg!(stop);
                        }
                        self.tasks_in_flight -= 1;
                    },
                    None => panic!("task end received before start... OH MY GOD !")
                },
                Err(error) => panic!("{}", error)
            }
        }
        anything_to_receive
    }
    pub fn start_task(&mut self, task:HT) {
        let threads = task.max_threads();
        //dbg!(task.clone(),self.tasks_in_flight);
        for i in 0..threads {
            self.tasks_in_flight += 1;
            if self.idle_threads > 0 {
                self.idle_threads -= 1;
            }
            match self.task_counter.get_mut(&task) {
                Some(counter) => {*counter += 1;},
                None => {self.task_counter.insert(task.clone(), 1);}
            }
            self.send.send(SchedulerTask::Task{tsk:task.clone(), thread_number:i, number_of_threads_on_task:threads}).expect("couldn't send task for some reason");
        }
        //dbg!(self.tasks_in_flight);
    }
    pub fn advance_all_sequences(&mut self) -> bool {
        let mut advanced_a_sequence = false;
        let mut seq_starts = Vec::with_capacity(2);
        let mut task_starts = Vec::with_capacity(self.current_tasks.tasks.len());
        for seq in &mut self.current_tasks.tasks {
            match seq.get_state() {
                SequenceState::Started => match seq.current_task() {
                    SequencedTask::StartSequence(id) => {seq_starts.push(id); seq.advance_sequence(); advanced_a_sequence = true;},
                    SequencedTask::StartTask(tsk) => {task_starts.push(tsk); seq.advance_sequence(); advanced_a_sequence = true;}
                    SequencedTask::WaitFor(tsk) => {
                        if !self.current_tasks.must_be_finished_before_next.contains(&tsk) {
                            self.current_tasks.must_be_finished_before_next.push(tsk.clone());
                        }
                        match self.task_counter.get(&tsk) {
                            Some(counter) => if *counter == 0 {seq.advance_sequence(); advanced_a_sequence = true;}
                            None => panic!("waiting for a task end before it started... THE HORROR !")
                        }
                    }
                    
                    
                },
                SequenceState::Finished | SequenceState::NotStarted => (),
            }
        }
        
        for start in seq_starts {
            if self.current_tasks.tasks[start].get_state() == SequenceState::NotStarted {
                self.current_tasks.tasks[start].start_sequence();
            }
            else {
                panic!("STARTED AN ALREADY STARTED OR FINISHED TASKKKK !!!!!");
            }
        }
        for start in task_starts {
            self.start_task(start);
        }
        advanced_a_sequence
    }
    pub fn end_threads(mut self) {
        for i in 0..self.number_of_threads {
            self.send.send(SchedulerTask::Stop).expect("couldn't send task for some reason");
        }
    }
}

fn task_thread<HT:HordeTask>(task_rcv:Receiver<SchedulerTask<HT>>, stop_send:Sender<HT>, mut data:HT::HTD) {
    loop {
        match task_rcv.recv() {
            Ok(sc_task) => match sc_task {
                SchedulerTask::Stop => break,
                SchedulerTask::Task{tsk, thread_number, number_of_threads_on_task} => {data.do_task(tsk.clone(), thread_number, number_of_threads_on_task); stop_send.send(tsk).expect("couldn't send stop womp womp"); }
            }
            Err(error) => panic!("grosse erreur dans thread de travail {}", error)
        }
    }
}

pub trait IndividualTask {
    type TD;
    type TID;
    fn do_task(&mut self, task_id:Self::TID, thread_number:usize, number_of_threads:usize);
}