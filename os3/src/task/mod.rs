use crate::config::{MAX_APP_NUM, MAX_SYSCALL_NUM};
use crate::loader::{get_num_app, init_app_cx};
use crate::sync::UPSafeCell;
use crate::timer::get_time_us;
use alloc::vec::Vec;
use lazy_static::lazy_static;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum TaskStatus {
    UInited,
    Ready,
    Running,
    Suspend,
    Exit,
}

#[derive(PartialEq, Clone, Copy)]
pub struct TaskContext {
    pub id: usize,
    pub context: usize,
    pub time: usize,
    pub status: TaskStatus,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
}

#[derive(PartialEq, Clone, Copy)]
struct TaskInner {
    cur_app: usize,
    apps: [TaskContext; MAX_APP_NUM],
}

pub struct TaskManager {
    app_num: usize,
    inner: UPSafeCell<TaskInner>,
}

impl TaskManager {
    pub fn init(&self) {
        let mut inner = self.inner.exclusive_access();
        for id in 0..self.app_num {
            inner.apps[id].status = TaskStatus::Ready;
        }
        drop(inner)
    }

    pub fn get_app_nums(&self) -> usize {
        self.app_num
    }

    pub fn get_cur_app_id(&self) -> usize {
        let inner = self.inner.exclusive_access();
        let cur_app = inner.cur_app;
        drop(inner);
        cur_app
    }

    pub fn syscall_count(&self, syscall_id: usize) {
        let mut inner = self.inner.exclusive_access();
        let cur_app = inner.cur_app;
        inner.apps[cur_app].syscall_times[syscall_id] += 1;
        drop(inner);
    }

    pub fn get_task_status(&self) -> TaskStatus {
        let inner = self.inner.exclusive_access();
        let cur_app = inner.cur_app;
        let status = inner.apps[cur_app].status;
        drop(inner);
        status
    }

    pub fn get_task_time(&self) -> usize {
        let inner = self.inner.exclusive_access();
        let cur_app = inner.cur_app;
        let time = inner.apps[cur_app].time;
        drop(inner);
        get_time_us() - time
    }

    pub fn get_syscall_count(&self, data: &mut [u32; MAX_SYSCALL_NUM]) {
        let inner = self.inner.exclusive_access();
        let cur_app = inner.cur_app;
        data.copy_from_slice(&inner.apps[cur_app].syscall_times);
        drop(inner);
    }

    pub fn run_app_by_id(&self, id: usize) {
        if id >= self.app_num {
            panic!("All applications was finished!");
        }

        let inner = self.inner.exclusive_access();
        let context = inner.apps[id].context;
        drop(inner);

        extern "C" {
            fn __restore(cx_addr: usize);
        }
        unsafe {
            __restore(context);
        }
        panic!("Unreachable in run_app_by_id");
    }

    pub fn suspend_and_run_next_app(&self, context: usize) {
        let mut inner = self.inner.exclusive_access();
        let index = inner.cur_app;
        inner.apps[index].status = TaskStatus::Ready;
        inner.apps[index].context = context;
        inner.cur_app = (inner.cur_app + 1) % self.app_num;
        drop(inner);
        self.run_next_app();
    }

    pub fn exit_and_run_next_app(&self) {
        let mut inner = self.inner.exclusive_access();
        let index = inner.cur_app;
        inner.apps[index].status = TaskStatus::Exit;
        drop(inner);
        self.run_next_app();
    }

    pub fn run_next_app(&self) {
        let mut inner = self.inner.exclusive_access();
        let index = inner.cur_app;
        let mut task_id: usize = self.app_num;

        //inner.apps[..self.app_num].into_iter().for_each(|x| {
        //    print!("{}: {:?} ", x.id, x.status);
        //});
        //println!(" ");

        if inner.apps[..self.app_num]
            .into_iter()
            .filter(|x| x.status == TaskStatus::Ready)
            .count()
            == 0
        {
            panic!("All applications were finished!")
        }

        for i in 0..self.app_num {
            if inner.apps[(index + i) % self.app_num].status == TaskStatus::Ready {
                task_id = (index + i) % self.app_num;
                break;
            }
        }

        inner.cur_app = task_id;
        if inner.apps[task_id].time == 0 {
            inner.apps[task_id].time = get_time_us();
        }
        inner.apps[task_id].status = TaskStatus::Running;
        drop(inner);
        info!("run app by id, {}", task_id);
        self.run_app_by_id(task_id);
        panic!("Unreachable in run_next_app");
    }
}

lazy_static! {
    static ref APP_MANAGER: TaskManager = unsafe {
        let num_app = get_num_app();
        let mut inner = TaskInner {
            cur_app: 0,
            apps: [TaskContext {
                id: 0,
                context: 0,
                time: 0,
                status: TaskStatus::UInited,
                syscall_times: [0; MAX_SYSCALL_NUM],
            }; MAX_APP_NUM],
        };

        for id in 0..num_app {
            inner.apps[id].id = id;
            inner.apps[id].context = init_app_cx(id);
        }
        TaskManager {
            app_num: num_app,
            inner: UPSafeCell::new(inner),
        }
    };
}

pub fn get_task_manager() -> &'static TaskManager {
    return &APP_MANAGER;
}
