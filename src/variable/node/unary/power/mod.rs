#[cfg(test)]
use super::{assert_almost_equals, new_backward_input, new_input, new_tensor};
use super::{
    expect_tensor, expect_tensor_mut, Backward, Cache, Data, Forward, Gradient, Overwrite, Tensor,
};
use ndarray::Zip;
use std::{
    cell::{Cell, Ref, RefCell, RefMut},
    fmt::{Debug, Display},
    rc::Rc,
};

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Power ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
pub struct Power<T: ?Sized>
where
    T: Data,
{
    operand: Rc<T>,
    data: RefCell<Tensor<T::Dim>>,
    exp: i32,
    computed: Cell<bool>,
}

impl<T: ?Sized> Power<T>
where
    T: Data,
{
    pub fn new(operand: Rc<T>, exp: i32) -> Self {
        let data = Tensor::zeros(operand.data().raw_dim());

        Self {
            operand,
            data: RefCell::new(data),
            exp,
            computed: Cell::new(false),
        }
    }
}

impl<T: ?Sized> Cache for Power<T>
where
    T: Data,
{
    fn was_computed(&self) -> bool {
        self.computed.get()
    }

    fn reset_computation(&self) {
        self.computed.set(false);
    }
}

impl<T: ?Sized> Forward for Power<T>
where
    T: Data,
{
    fn forward(&self) {
        if self.was_computed() {
            return;
        }

        self.computed.set(true);
        let exp = self.exp;
        Zip::from(&mut *self.data.borrow_mut())
            .and(&*self.operand.data())
            .for_each(|v, o| *v = o.powi(exp));
    }
}

impl<T: ?Sized> Data for Power<T>
where
    T: Data,
{
    type Dim = T::Dim;

    fn data(&self) -> Ref<Tensor<Self::Dim>> {
        self.data.borrow()
    }

    fn data_mut(&self) -> RefMut<Tensor<Self::Dim>> {
        self.data.borrow_mut()
    }
}

impl<T: ?Sized> Debug for Power<T>
where
    T: Data,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Power")
            .field("data", &self.data.borrow())
            .field("exp", &self.exp)
            .field("computed", &self.computed.get())
            .finish()
    }
}

impl<T: ?Sized> Display for Power<T>
where
    T: Data,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", &self.data.borrow())
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ PowerBackward ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

pub struct PowerBackward<T: ?Sized, U: ?Sized>
where
    T: Gradient,
    U: Data<Dim = T::Dim>,
{
    gradient: RefCell<Option<Tensor<T::Dim>>>,
    shape: T::Dim,
    overwrite: Cell<bool>,
    diff_operand: Rc<T>,
    no_diff_operand: Rc<U>,
    exp: i32,
}

impl<T: ?Sized, U: ?Sized> PowerBackward<T, U>
where
    T: Gradient,
    U: Data<Dim = T::Dim>,
{
    pub fn new(diff_operand: Rc<T>, no_diff_operand: Rc<U>, exp: i32) -> Self {
        let shape = diff_operand.gradient().raw_dim();

        Self {
            gradient: RefCell::new(Some(Tensor::zeros(shape.clone()))),
            shape,
            overwrite: Cell::new(true),
            diff_operand,
            no_diff_operand,
            exp,
        }
    }
}

impl<T: ?Sized, U: ?Sized> Gradient for PowerBackward<T, U>
where
    T: Gradient,
    U: Data<Dim = T::Dim>,
{
    type Dim = T::Dim;

    fn gradient(&self) -> Ref<Tensor<Self::Dim>> {
        expect_tensor(&self.gradient)
    }

    fn gradient_mut(&self) -> RefMut<Tensor<Self::Dim>> {
        expect_tensor_mut(&self.gradient)
    }
}

impl<T: ?Sized, U: ?Sized> Overwrite for PowerBackward<T, U>
where
    T: Gradient,
    U: Data<Dim = T::Dim>,
{
    fn can_overwrite(&self) -> bool {
        self.overwrite.get()
    }

    fn set_overwrite(&self, state: bool) {
        self.overwrite.set(state);
    }
}

impl<T: ?Sized, U: ?Sized> Backward for PowerBackward<T, U>
where
    T: Gradient,
    U: Data<Dim = T::Dim>,
{
    fn backward(&self) {
        let mut op_grad = self.diff_operand.gradient_mut();
        let op_data = self.no_diff_operand.data();
        let grad = self.gradient();
        let exp = self.exp;

        let zip = Zip::from(&mut *op_grad).and(&*grad).and(&*op_data);
        if self.diff_operand.can_overwrite() {
            zip.for_each(|op_grad_el, grad_el, op_data_el| {
                *op_grad_el = grad_el * op_data_el.powi(exp - 1) * exp as f32
            });
            self.diff_operand.set_overwrite(false);
        } else {
            zip.for_each(|op_grad_el, grad_el, op_data_el| {
                *op_grad_el += grad_el * op_data_el.powi(exp - 1) * exp as f32
            });
        }
    }

    fn no_grad(&self) {
        *self.gradient.borrow_mut() = None;
    }

    fn with_grad(&self) {
        *self.gradient.borrow_mut() = Some(Tensor::zeros(self.shape.clone()));
    }
}

impl<T: ?Sized, U: ?Sized> Debug for PowerBackward<T, U>
where
    T: Gradient,
    U: Data<Dim = T::Dim>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PowerBackward")
            .field("gradient", &self.gradient.borrow())
            .field("overwrite", &self.overwrite.get())
            .finish()
    }
}

impl<T: ?Sized, U: ?Sized> Display for PowerBackward<T, U>
where
    T: Gradient,
    U: Data<Dim = T::Dim>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match &*self.gradient.borrow() {
            Some(gradient) => write!(f, "{}", &gradient),
            None => write!(f, "None"),
        }
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~ Tests ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
#[cfg(test)]
mod test;
