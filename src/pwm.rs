use core::marker::PhantomData;
use crate::stm32::{TIM3, TIM8};
use embedded_hal::PwmPin;
use super::gpio::{AF4, AF10};
use super::gpio::gpioc::{PC8};
use super::gpio::gpiob::{PB9};
use crate::rcc::{Clocks};
use crate::stm32::{RCC};

//pub struct Tim1Ch1 {}
//pub struct Tim1Ch2 {}
//pub struct Tim1Ch3 {}
//pub struct Tim1Ch4 {}

//pub struct Tim3Ch1 {}
//pub struct Tim3Ch2 {}
pub struct Tim3Ch3 {}
//pub struct Tim3Ch4 {}

//pub struct Tim8Ch1 {}
//pub struct Tim8Ch2 {}
pub struct Tim8Ch3 {}
//pub struct Tim8Ch4 {}

pub struct NoPins {}
pub struct WithPins {}

pub struct PwmChannel<X, T> {
    pub(crate) timx_chx: PhantomData<X>,
    pub(crate) pin_status: PhantomData<T>,
}

macro_rules! pwm_timer_private {
    // TODO: TimxChy needs to become a list
    ($timx:ident, $TIMx:ty, $apbxenr:ident, $timxen:ident, $trigger_update_event:expr, $enable_break_timer:expr, $TimxChy:ident) => {
        pub fn $timx(tim: $TIMx, res: u16, freq: u16, clocks: &Clocks) -> PwmChannel<$TimxChy, NoPins> {
            // Power the timer
            // We use unsafe here to abstract away this implementation detail
            // Justification: It is safe because only scopes with mutable references
            // to TIMx should ever modify this bit.
            unsafe {
                &(*RCC::ptr()).$apbxenr.modify(|_, w| w.$timxen().set_bit());
            }

            // enable auto reload preloader
            tim.cr1.write(|w| w.arpe().set_bit());

            // Set the "resolution" of the duty cycle (ticks before restarting at 0)
            tim.arr.write(|w| w.arr().bits(res));
            // TODO: Use Hertz?
            // Set the pre-scaler
            tim.psc.write(|w| w.psc().bits(clocks.pclk2().0 as u16 / (res * freq)));

            // Make the settings reload immediately for TIM1/8
            $trigger_update_event(&tim);

            tim.smcr.write(|w| w); // Reset the slave/master config
            tim.cr2.write(|w| w); // reset

            // TODO: Not all timers have 4 channels, so these need to be in the macro
            tim.ccmr1_output().write(|w| w
                // Select PWM Mode 1 for CH1/CH2
                .oc1m().bits(0b0110)
                .oc2m().bits(0b0110)
                // set pre-load enable so that updates to the duty cycle
                // propagate but _not_ in the middle of a cycle.
                .oc1pe().set_bit()
                .oc2pe().set_bit()
            );
            tim.ccmr2_output().write(|w| w
                // Select PWM Mode 1 for CH3/CH4
                .oc3m().bits(0b0110)
                .oc4m().bits(0b0110)
                // set pre-load enable so that updates to the duty cycle
                // propagate but _not_ in the middle of a cycle.
                .oc3pe().set_bit()
                .oc4pe().set_bit()
            );

            // Enable outputs (STM32 Break Timer Specific)
            $enable_break_timer(&tim);

            // Enable the Timer
            tim.cr1.modify(|_, w| w.cen().set_bit());

            // TODO: This should return all four channels
            PwmChannel { timx_chx: PhantomData, pin_status: PhantomData }
        }
    }
}

macro_rules! pwm_timer_basic {
    ($timx:ident, $TIMx:ty, $apbxenr:ident, $timxen:ident, $TimxChy:ident) => {
        pwm_timer_private!(
            $timx,
            $TIMx,
            $apbxenr,
            $timxen,
            |_| (),
            |_| (),
            $TimxChy
        );
    }
}

macro_rules! pwm_timer_advanced {
    ($timx:ident, $TIMx:ty, $apbxenr:ident, $timxen:ident, $TimxChy:ident) => {
        pwm_timer_private!(
            $timx,
            $TIMx,
            $apbxenr,
            $timxen,
            |tim: &$TIMx| tim.egr.write(|w| w.ug().set_bit()),
            |tim: &$TIMx| tim.bdtr.write(|w| w.moe().set_bit()),
            $TimxChy
        );
    }
}

pwm_timer_basic!(tim3, TIM3, apb1enr, tim3en, Tim3Ch3);
pwm_timer_advanced!(tim8, TIM8, apb2enr, tim8en, Tim8Ch3);



macro_rules! pwm_channel_pin {
    ($TimiChi:ident, $output_to_pxi:ident, $PXi:ident, $AFi:ident) => {
        impl<T> PwmChannel<$TimiChi, T> {
            pub fn $output_to_pxi(self, _p: $PXi<$AFi>) -> PwmChannel<$TimiChi, WithPins> {
                PwmChannel { timx_chx: PhantomData, pin_status: PhantomData }
            }
        }
    }
}

pwm_channel_pin!(Tim8Ch3, output_to_pc8, PC8, AF4);
pwm_channel_pin!(Tim8Ch3, output_to_pb9, PB9, AF10);

impl PwmPin for PwmChannel<Tim8Ch3, WithPins> {
    type Duty = u16;

    fn disable(&mut self) {
        unsafe {
            &(*TIM8::ptr()).ccer.modify(|_, w| w.cc3e().clear_bit());
        }
    }

    fn enable(&mut self) {
        unsafe {
            &(*TIM8::ptr()).ccer.modify(|_, w| w.cc3e().set_bit());
        }
    }

    fn get_max_duty(&self) -> Self::Duty {
        unsafe {
            // TODO: should the resolution just be stored in the channel rather than read?
            // This would work if it changed, but isn't it the point that it can't be?
            (*TIM8::ptr()).arr.read().arr().bits()
        }
    }

    fn get_duty(&self) -> Self::Duty {
        unsafe {
            // TODO: This could theoretically be passed into the PwmChannel struct
            (*TIM8::ptr()).ccr3.read().ccr().bits()
        }
    }

    fn set_duty(&mut self, duty: Self::Duty) -> () {
        unsafe {
            // TODO: This could theoretically be passed into the PwmChannel struct
            // and it would then be safe to modify
            &(*TIM8::ptr()).ccr3.modify(|_, w| w.ccr().bits(duty));
        }
    }
}
