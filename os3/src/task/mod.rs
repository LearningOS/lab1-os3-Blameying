use crate::config::{MAX_APP_NUM, MAX_SYSCALL_NUM};
use crate::loader::{get_num_app, init_app_cx};
use crate::sync::UPSafeCell;
use lazy_static::lazy_static;

#[derive(PartialEq, Clone, Copy)]
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
    pub size: usize,
    pub sepc: usize,
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
        for id in 0..inner.apps.len() {
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

    pub fn run_app_by_id(&self, id: usize) {
        info!("running id: {}", id);
        if id >= self.app_num {
            panic!("All applications was finished!");
        }

        let inner = self.inner.exclusive_access();
        let context = inner.apps[id].context;
        drop(inner);

        extern "C" {
            fn __restore(cx_addr: usize);
        }
        info!("restore !");
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

    pub fn run_next_app(&self) {
        let mut inner = self.inner.exclusive_access();
        let index = inner.cur_app;
        let mut task_id: Option<usize> = None;

        if index == 0 && (inner.apps[index].status == TaskStatus::Ready) {
            task_id = Some(index);
        } else {
            for item in inner.apps[index..].into_iter().enumerate() {
                let (id, _) = item;
                if inner.apps[id].status == TaskStatus::Ready {
                    task_id = Some(id);
                    break;
                }
            }
            if task_id == None {
                for item in inner.apps[..index].into_iter().enumerate() {
                    let (id, _) = item;
                    if inner.apps[id].status == TaskStatus::Ready {
                        task_id = Some(id);
                        break;
                    }
                }
            }
        }

        if let Some(id) = task_id {
            if id != index {
                inner.apps[index].status = TaskStatus::Exit;
            }
            inner.apps[id].status = TaskStatus::Running;
            drop(inner);
            info!("run app by id, {}", id);
            self.run_app_by_id(id);
        } else {
            drop(inner);
        }
        panic!("All applications was finished!");
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
                size: 0,
                sepc: 0,
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
