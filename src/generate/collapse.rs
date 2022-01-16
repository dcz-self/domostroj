/*
 * SPDX-License-Identifier: LGPL-3.0-or-later
 */
use crate::edit;
use baustein::re::{ ConstAnyShape, ConstShape };
use rental::rental;
use rental::RentalError;
use wfc_3d as wfc;
use wfc::stamp::{gather_stamps, StampCollection, StampSpace, Wrapping};


type StampShape = ConstAnyShape<3, 3, 3>;


rental! {
    mod stamps {
        use super::*;

        #[rental]
        pub struct Stamps {
            source: Box<StampSpace<edit::Shape>>,
            stamps: StampCollection<'source, StampShape, edit::Shape>
        }
    }
}

use stamps::Stamps;

impl Stamps {
    fn from(source: StampSpace<edit::Shape>) -> Stamps {
        Self::try_new(
            Box::new(source),
            |source| {
                let stamps = gather_stamps::<_, _>(&*source, Wrapping);
                Ok(StampCollection::from_iter(stamps))
            }
        ).unwrap_or_else(|_: RentalError<(), _>| panic!("Failed"))
    }
}

