
use std::{future::Future, sync::RwLock, ops::DerefMut};

pub struct Vm {
    cycle: u32,
    pub reg_x: i32,
}

struct CycleYield(usize);
impl Future for CycleYield {
    type Output = ();

    fn poll(mut self: std::pin::Pin<&mut Self>, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {

        if self.0 > 0 {
            self.0 -= 1;
            std::task::Poll::Pending
        } else {
            std::task::Poll::Ready(())
        }
    }
}

pub async fn yield_cycles(num_cycles: usize) {
    CycleYield(num_cycles).await
}

impl Vm {
    
    pub fn get_cycle(&self) -> u32 {self.cycle}

    /// Execute a series of instructions in a virtual machine.
    /// Note that the CPU interpreter is passed an `RwLock` to the CPU state.
    /// This lock cannot be held across awaits, and the application will panic if this happens.
    pub fn execute<
        'vm,
        'program: 'vm,
        CpuInterpretFuture: Future<Output = ()> + 'program,
        CpuInterpretFn: Fn(&'vm RwLock<Vm>, crate::Instruction) -> CpuInterpretFuture,
        CycleFn: FnMut(&mut Vm),
    >(
        instructions: &'program [crate::Instruction],
        interpret_fn: CpuInterpretFn,
        mut cycle_fn: CycleFn,
    ) -> Result<(), color_eyre::Report> {
        let vm = RwLock::new(Vm {
            cycle: 0,
            reg_x: 1,
        });

        let process_future = {
            // Safety: This function blocks on the async block, so it cannot be
            //         called after this function ends, when the VM is dropped.
            //         I'm sure there's a way to wrangle these lifetimes without
            //         unsafe, but alas, I'm a novice at async.
            let vm: &'static RwLock<Vm> = unsafe { std::mem::transmute(&vm) };
            async move {
                for i in instructions {
                    interpret_fn(vm, i.clone()).await;
                }
            }
        };

        futures::pin_mut!(process_future);


        // Can you tell this is one of my first forays into async? haha

        const DUMMY_VTABLE: std::task::RawWakerVTable = std::task::RawWakerVTable::new(
            |_ctx| {unimplemented!()},
            |_ctx| {unimplemented!()},
            |_ctx| {unimplemented!()},
            |_ctx| {},
        );
        let dummy_waker = std::task::RawWaker::new(std::ptr::null(), &DUMMY_VTABLE);
        let waker = unsafe { std::task::Waker::from_raw(dummy_waker) };
        let mut future_ctx = std::task::Context::from_waker(&waker);


        loop {
            let status = process_future.as_mut().poll(&mut future_ctx);

            let mut mut_vm = vm.try_write().map_err(|err| {
                color_eyre::eyre::eyre!("CPU emulator held on to register lock! {:?}", err.to_string())
            })?;
            cycle_fn(mut_vm.deref_mut());

            mut_vm.cycle += 1;

            if status.is_ready() {
                break;
            }
        }

        Ok(())
    }
}