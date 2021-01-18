use super::{PartitionWriter, Writer};
use crate::errors::{ConnectorAgentError, Result};
use crate::types::{DataType, DataTypeCheck};
use fehler::throw;
use ndarray::{Array2, ArrayViewMut2, Axis};
use std::mem::{size_of, transmute};
use std::ptr::copy_nonoverlapping;

#[derive(Clone)]
pub struct DummyWriter {
    nrows: usize,
    schema_: Vec<DataType>,
    pub buffer: Array2<u64>,
}

impl Writer for DummyWriter {
    type PartitionWriter<'a> = DummyPartitionWriter<'a>;

    fn allocate(nrows: usize, schema: Vec<DataType>) -> Self {
        let ncols = schema.len();
        DummyWriter {
            nrows,
            schema_: schema,
            buffer: Array2::zeros((nrows, ncols)),
        }
    }

    fn partition_writer<'a>(&'a mut self, counts: &[usize]) -> Vec<Self::PartitionWriter<'a>> {
        assert_eq!(counts.iter().sum::<usize>(), self.nrows);
        let schema = self.schema().to_vec();

        let mut mut_view = self.buffer.view_mut();
        let mut ret = vec![];
        for &c in counts {
            let (splitted, rest) = mut_view.split_at(Axis(0), c);
            mut_view = rest;
            ret.push(DummyPartitionWriter::new(splitted, schema.clone()));
        }
        ret
    }

    fn schema(&self) -> &[DataType] {
        self.schema_.as_slice()
    }
}

pub struct DummyPartitionWriter<'a> {
    buffer: ArrayViewMut2<'a, u64>,
    schema: Vec<DataType>,
}

impl<'a> PartitionWriter<'a> for DummyPartitionWriter<'a> {
    unsafe fn write<T>(&mut self, row: usize, col: usize, value: T) {
        let target: *mut T = transmute(self.buffer.uget_mut((row, col)));
        copy_nonoverlapping(&value, target, 1);
    }

    fn write_safe<T>(&mut self, row: usize, col: usize, value: T) -> Result<()>
    where
        DataType: DataTypeCheck<T>,
    {
        assert_eq!(size_of::<T>(), size_of::<u64>());
        if !self.schema[col].verify() {
            throw!(ConnectorAgentError::DataTypeCheckFailed(self.schema[col], DataType::found()));
        }
        unsafe { self.write(row, col, value) };
        Ok(())
    }

    fn nrows(&self) -> usize {
        self.buffer.nrows()
    }

    fn ncols(&self) -> usize {
        self.buffer.ncols()
    }
}

impl<'a> DummyPartitionWriter<'a> {
    fn new(buffer: ArrayViewMut2<'a, u64>, schema: Vec<DataType>) -> Self {
        Self { buffer, schema }
    }
}