


pub struct ExponentialSmoother {
    time_constant: f64,
    value: f64
}


impl ExponentialSmoother {

    pub fn new(time_constant: f64) -> Self {
        assert!(time_constant.is_finite());
        assert!(time_constant > 0.0);
        ExponentialSmoother {
            time_constant: time_constant,
            value: 0.0,
        }
    }

    pub fn get(&self) -> f64 {
        self.value
    }

    pub fn add(&mut self, value: f64) {
        self.value = ((self.time_constant - 1.0) * self.value + value) / self.time_constant;
    }

    pub fn reset(&mut self) {
        self.value = 0.0;
    }

}


pub struct ExponentialSmootherWithRunUpPeriod {
    time_constant: f64,    
    run_up_period: f64,
    time: f64,
    value: f64
}


impl ExponentialSmootherWithRunUpPeriod {

    pub fn new(time_constant: f64, run_up_period: f64) -> Self {
        ExponentialSmootherWithRunUpPeriod {
            time_constant: time_constant,
            run_up_period: run_up_period,
            time: 0.0,
            value: 0.0,
        }
    }

    pub fn get(&self) -> f64 {
        self.value
    }

    pub fn add(&mut self, value: f64) {
        self.time += 1.0;
        let t = if self.time <= self.run_up_period {
            self.time
        } else {
            self.time_constant
        };
        self.value = ((t - 1.0) * self.value + value) / t;
    }

    pub fn reset(&mut self) {
        self.time = 0.0;
        self.value = 0.0;
    }

}