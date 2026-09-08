use runenui_core::{
    ImageFit, ImageMapping, ImagePaintDescriptor, ImagePrimitive, LogicalRect,
    PaintContributionItem, PaintPrimitive,
};

pub(super) fn publication_primitive(item: &PaintContributionItem) -> PaintPrimitive {
    match item.primitive() {
        PaintPrimitive::Image(image) => {
            let descriptor = image.authored_descriptor().unwrap_or_else(|| {
                unreachable!("paint contributions cannot author runtime-resolved image primitives")
            });
            PaintPrimitive::Image(resolve_image(descriptor))
        }
        primitive => primitive.clone(),
    }
}

fn resolve_image(descriptor: &ImagePaintDescriptor) -> ImagePrimitive {
    let image = descriptor.image();
    let patches = match descriptor.mapping() {
        ImageMapping::Fit {
            crop,
            alignment,
            fit,
        } => resolve_fit(
            image.intrinsic_size().width(),
            image.intrinsic_size().height(),
            crop.x().get(),
            crop.y().get(),
            crop.width().get(),
            crop.height().get(),
            alignment.x().get(),
            alignment.y().get(),
            fit,
            descriptor.destination(),
        ),
        ImageMapping::NineSlice {
            source,
            source_insets,
            destination_insets,
        } => resolve_nine_slice(
            image.intrinsic_size().width(),
            image.intrinsic_size().height(),
            source.x().get(),
            source.y().get(),
            source.width().get(),
            source.height().get(),
            [
                source_insets.top(),
                source_insets.right(),
                source_insets.bottom(),
                source_insets.left(),
            ],
            [
                destination_insets.top().get(),
                destination_insets.right().get(),
                destination_insets.bottom().get(),
                destination_insets.left().get(),
            ],
            descriptor.destination(),
        ),
    };
    ImagePrimitive::__runtime_resolved(
        image.resource_ref().clone(),
        image.intrinsic_size(),
        patches,
    )
    .unwrap_or_else(|| unreachable!("validated image policy resolves within intrinsic bounds"))
}

#[allow(
    clippy::cast_precision_loss,
    reason = "intrinsic u32 pixel extents become finite neutral f32 source geometry only at the runtime publication boundary"
)]
#[allow(
    clippy::too_many_arguments,
    reason = "the pure image mapping helper keeps every authored source/alignment/fit input explicit"
)]
fn resolve_fit(
    intrinsic_width: u32,
    intrinsic_height: u32,
    crop_x: f32,
    crop_y: f32,
    crop_width: f32,
    crop_height: f32,
    align_x: f32,
    align_y: f32,
    fit: ImageFit,
    destination: LogicalRect,
) -> Vec<([f32; 4], LogicalRect)> {
    if destination.width() == 0.0 || destination.height() == 0.0 {
        return Vec::new();
    }

    let intrinsic_width = intrinsic_width as f32;
    let intrinsic_height = intrinsic_height as f32;
    let source_x = intrinsic_width * crop_x;
    let source_y = intrinsic_height * crop_y;
    let source_width = intrinsic_width * crop_width;
    let source_height = intrinsic_height * crop_height;

    let destination_width = destination.width();
    let destination_height = destination.height();
    let (rendered_width, rendered_height) = match fit {
        ImageFit::Fill => (destination_width, destination_height),
        ImageFit::Contain => {
            let scale = (destination_width / source_width).min(destination_height / source_height);
            (source_width * scale, source_height * scale)
        }
        ImageFit::Cover => {
            let scale = (destination_width / source_width).max(destination_height / source_height);
            (source_width * scale, source_height * scale)
        }
        ImageFit::None => (source_width, source_height),
        ImageFit::ScaleDown => {
            if source_width <= destination_width && source_height <= destination_height {
                (source_width, source_height)
            } else {
                let scale =
                    (destination_width / source_width).min(destination_height / source_height);
                (source_width * scale, source_height * scale)
            }
        }
    };

    let rendered_x = destination.x() + (destination_width - rendered_width) * align_x;
    let rendered_y = destination.y() + (destination_height - rendered_height) * align_y;
    let visible_x0 = destination.x().max(rendered_x);
    let visible_y0 = destination.y().max(rendered_y);
    let visible_x1 = (destination.x() + destination_width).min(rendered_x + rendered_width);
    let visible_y1 = (destination.y() + destination_height).min(rendered_y + rendered_height);
    if visible_x1 <= visible_x0 || visible_y1 <= visible_y0 {
        return Vec::new();
    }

    let u0 = (visible_x0 - rendered_x) / rendered_width;
    let v0 = (visible_y0 - rendered_y) / rendered_height;
    let u1 = (visible_x1 - rendered_x) / rendered_width;
    let v1 = (visible_y1 - rendered_y) / rendered_height;
    let source = [
        source_x + source_width * u0,
        source_y + source_height * v0,
        source_width * (u1 - u0),
        source_height * (v1 - v0),
    ];
    let visible = LogicalRect::try_new(
        visible_x0,
        visible_y0,
        visible_x1 - visible_x0,
        visible_y1 - visible_y0,
    )
    .unwrap_or_else(|_| unreachable!("resolved visible image destination remains finite"));
    vec![(source, visible)]
}

#[allow(
    clippy::cast_precision_loss,
    reason = "intrinsic u32 pixel extents become finite neutral f32 source geometry only at the runtime publication boundary"
)]
#[allow(
    clippy::too_many_arguments,
    reason = "the pure nine-slice helper keeps the exact authored source and destination edge facts explicit"
)]
fn resolve_nine_slice(
    intrinsic_width: u32,
    intrinsic_height: u32,
    crop_x: f32,
    crop_y: f32,
    crop_width: f32,
    crop_height: f32,
    source_insets: [f32; 4],
    destination_insets: [f32; 4],
    destination: LogicalRect,
) -> Vec<([f32; 4], LogicalRect)> {
    if destination.width() == 0.0 || destination.height() == 0.0 {
        return Vec::new();
    }

    let intrinsic_width = intrinsic_width as f32;
    let intrinsic_height = intrinsic_height as f32;
    let source_x = intrinsic_width * crop_x;
    let source_y = intrinsic_height * crop_y;
    let source_width = intrinsic_width * crop_width;
    let source_height = intrinsic_height * crop_height;
    let [source_top, source_right, source_bottom, source_left] = source_insets;
    let [destination_top, destination_right, destination_bottom, destination_left] =
        destination_insets;
    let (destination_left, destination_right) = normalize_pair(
        destination_left,
        destination_right,
        destination.width(),
    );
    let (destination_top, destination_bottom) = normalize_pair(
        destination_top,
        destination_bottom,
        destination.height(),
    );

    let source_xs = [
        source_x,
        source_x + source_left,
        source_x + source_width - source_right,
        source_x + source_width,
    ];
    let source_ys = [
        source_y,
        source_y + source_top,
        source_y + source_height - source_bottom,
        source_y + source_height,
    ];
    let destination_xs = [
        destination.x(),
        destination.x() + destination_left,
        destination.x() + destination.width() - destination_right,
        destination.x() + destination.width(),
    ];
    let destination_ys = [
        destination.y(),
        destination.y() + destination_top,
        destination.y() + destination.height() - destination_bottom,
        destination.y() + destination.height(),
    ];

    let mut patches = Vec::with_capacity(9);
    for row in 0..3 {
        for column in 0..3 {
            let source_patch_width = source_xs[column + 1] - source_xs[column];
            let source_patch_height = source_ys[row + 1] - source_ys[row];
            let destination_patch_width = destination_xs[column + 1] - destination_xs[column];
            let destination_patch_height = destination_ys[row + 1] - destination_ys[row];
            if source_patch_width <= 0.0
                || source_patch_height <= 0.0
                || destination_patch_width <= 0.0
                || destination_patch_height <= 0.0
            {
                continue;
            }
            let destination_patch = LogicalRect::try_new(
                destination_xs[column],
                destination_ys[row],
                destination_patch_width,
                destination_patch_height,
            )
            .unwrap_or_else(|_| unreachable!("resolved nine-slice destination remains finite"));
            patches.push((
                [
                    source_xs[column],
                    source_ys[row],
                    source_patch_width,
                    source_patch_height,
                ],
                destination_patch,
            ));
        }
    }
    patches
}

fn normalize_pair(first: f32, second: f32, available: f32) -> (f32, f32) {
    let sum = first + second;
    if sum <= available || sum == 0.0 {
        (first, second)
    } else {
        let scale = available / sum;
        (first * scale, second * scale)
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        ImageAlignment, ImageCrop, ImageDescriptor, ImageDestinationInsets, ImageFit,
        ImageIntrinsicSize, ImageMapping, ImagePaintDescriptor, ImageSourceInsets, LogicalLength,
        LogicalRect, PaintContributionItem, PaintPrimitive, ResourceKind, ResourceRef,
        UnitInterval,
    };

    use super::publication_primitive;

    fn rect(x: f32, y: f32, width: f32, height: f32) -> LogicalRect {
        LogicalRect::try_new(x, y, width, height)
            .unwrap_or_else(|_| unreachable!("fixture rectangle is valid"))
    }

    fn image(mapping: ImageMapping, destination: LogicalRect) -> PaintContributionItem {
        let descriptor = ImageDescriptor::new(
            ResourceRef::new(ResourceKind::Image),
            ImageIntrinsicSize::new(100, 50)
                .unwrap_or_else(|| unreachable!("fixture extent is non-zero")),
        )
        .unwrap_or_else(|_| unreachable!("fixture ref is image-kind"));
        PaintContributionItem::image(
            ImagePaintDescriptor::new(descriptor, destination, mapping)
                .unwrap_or_else(|_| unreachable!("fixture mapping is valid")),
        )
    }

    fn resolved(item: &PaintContributionItem) -> runenui_core::ImagePrimitive {
        let PaintPrimitive::Image(image) = publication_primitive(item) else {
            unreachable!("fixture resolves to image")
        };
        image
    }

    #[test]
    fn contain_and_cover_resolve_before_publication() {
        let contain = resolved(&image(
            ImageMapping::Fit {
                crop: ImageCrop::FULL,
                alignment: ImageAlignment::CENTER,
                fit: ImageFit::Contain,
            },
            rect(0.0, 0.0, 100.0, 100.0),
        ));
        assert_eq!(contain.resolved_patch_count(), Some(1));
        let (source, destination) = contain
            .resolved_patch(0)
            .unwrap_or_else(|| unreachable!("contain has one patch"));
        assert_eq!(source, [0.0, 0.0, 100.0, 50.0]);
        assert_eq!(destination, rect(0.0, 25.0, 100.0, 50.0));

        let cover = resolved(&image(
            ImageMapping::Fit {
                crop: ImageCrop::FULL,
                alignment: ImageAlignment::CENTER,
                fit: ImageFit::Cover,
            },
            rect(0.0, 0.0, 100.0, 100.0),
        ));
        let (source, destination) = cover
            .resolved_patch(0)
            .unwrap_or_else(|| unreachable!("cover has one patch"));
        assert_eq!(source, [25.0, 0.0, 50.0, 50.0]);
        assert_eq!(destination, rect(0.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn none_and_scale_down_use_exact_alignment_and_fit_rules() {
        let left = UnitInterval::ZERO;
        let bottom = UnitInterval::ONE;
        let none = resolved(&image(
            ImageMapping::Fit {
                crop: ImageCrop::FULL,
                alignment: ImageAlignment::new(left, bottom),
                fit: ImageFit::None,
            },
            rect(10.0, 20.0, 60.0, 30.0),
        ));
        let (source, destination) = none
            .resolved_patch(0)
            .unwrap_or_else(|| unreachable!("none has visible patch"));
        assert_eq!(source, [0.0, 20.0, 60.0, 30.0]);
        assert_eq!(destination, rect(10.0, 20.0, 60.0, 30.0));

        let scale_down = resolved(&image(
            ImageMapping::Fit {
                crop: ImageCrop::FULL,
                alignment: ImageAlignment::CENTER,
                fit: ImageFit::ScaleDown,
            },
            rect(0.0, 0.0, 50.0, 50.0),
        ));
        let (_, destination) = scale_down
            .resolved_patch(0)
            .unwrap_or_else(|| unreachable!("scale-down has one patch"));
        assert_eq!(destination, rect(0.0, 12.5, 50.0, 25.0));
    }

    #[test]
    fn crop_is_resolved_to_intrinsic_pixel_source_geometry() {
        let half = UnitInterval::HALF;
        let crop = ImageCrop::new(UnitInterval::ZERO, UnitInterval::ZERO, half, half)
            .unwrap_or_else(|_| unreachable!("fixture crop is valid"));
        let image = resolved(&image(
            ImageMapping::Fit {
                crop,
                alignment: ImageAlignment::CENTER,
                fit: ImageFit::Fill,
            },
            rect(1.0, 2.0, 40.0, 20.0),
        ));
        assert_eq!(
            image.resolved_patch(0),
            Some(([0.0, 0.0, 50.0, 25.0], rect(1.0, 2.0, 40.0, 20.0)))
        );
    }

    #[test]
    fn nine_slice_normalizes_destination_edges_and_keeps_row_major_patches() {
        let source_insets = ImageSourceInsets::new(10.0, 20.0, 10.0, 20.0)
            .unwrap_or_else(|_| unreachable!("fixture source insets are valid"));
        let length = |value| {
            LogicalLength::new(value)
                .unwrap_or_else(|_| unreachable!("fixture destination inset is valid"))
        };
        let image = resolved(&image(
            ImageMapping::NineSlice {
                source: ImageCrop::FULL,
                source_insets,
                destination_insets: ImageDestinationInsets::new(
                    length(20.0),
                    length(40.0),
                    length(20.0),
                    length(40.0),
                ),
            },
            rect(0.0, 0.0, 40.0, 20.0),
        ));
        assert_eq!(image.resolved_patch_count(), Some(4));
        assert_eq!(
            image.resolved_patch(0),
            Some(([0.0, 0.0, 20.0, 10.0], rect(0.0, 0.0, 20.0, 10.0)))
        );
        assert_eq!(
            image.resolved_patch(1),
            Some(([80.0, 0.0, 20.0, 10.0], rect(20.0, 0.0, 20.0, 10.0)))
        );
        assert_eq!(
            image.resolved_patch(2),
            Some(([0.0, 40.0, 20.0, 10.0], rect(0.0, 10.0, 20.0, 10.0)))
        );
        assert_eq!(
            image.resolved_patch(3),
            Some(([80.0, 40.0, 20.0, 10.0], rect(20.0, 10.0, 20.0, 10.0)))
        );
    }

    #[test]
    fn zero_destination_publishes_no_image_patches() {
        let image = resolved(&image(ImageMapping::default(), rect(0.0, 0.0, 0.0, 10.0)));
        assert_eq!(image.resolved_patch_count(), Some(0));
    }
}
