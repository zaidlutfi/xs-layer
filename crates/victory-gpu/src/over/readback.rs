//! GPU readback for the two Series S display passes.

use super::*;

/// Stretches a blue 3 by 2 scene to 5 by 3 and reads the result back, `None`
/// without a device. `padded`: the scene is the top left of a 4 by 3 texture
/// whose other texels are red, and the interface over it is empty; otherwise
/// the interface has a half green and a red strip.
fn stretched(padded: bool) -> Option<[[u8; 20]; 3]> {
    use crate::host::{host, transition};
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_R8G8B8A8_UNORM as RGBA8;
    let Some(host) = host() else {
        eprintln!("no D3D12 device; skipping display readback");
        return None;
    };
    let texture = |width: u32, height, flags, state| {
        let mut resource: Option<ID3D12Resource> = None;
        // SAFETY: plain creation of a texture with one RGBA8 subresource.
        unsafe {
            host.device
                .CreateCommittedResource(
                    &D3D12_HEAP_PROPERTIES {
                        Type: D3D12_HEAP_TYPE_DEFAULT,
                        ..Default::default()
                    },
                    D3D12_HEAP_FLAG_NONE,
                    &D3D12_RESOURCE_DESC {
                        Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                        Width: u64::from(width),
                        Height: height,
                        DepthOrArraySize: 1,
                        MipLevels: 1,
                        Format: RGBA8,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        Flags: flags,
                        ..Default::default()
                    },
                    state,
                    None,
                    &mut resource,
                )
                .unwrap();
        }
        resource.unwrap()
    };
    let sub = |r: &ID3D12Resource| D3D12_TEXTURE_COPY_LOCATION {
        // SAFETY: the copy location borrows this resource.
        pResource: unsafe { std::mem::transmute_copy(r) },
        Type: D3D12_TEXTURE_COPY_TYPE_SUBRESOURCE_INDEX,
        Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
            SubresourceIndex: 0,
        },
    };
    let rows = |r: &ID3D12Resource| D3D12_TEXTURE_COPY_LOCATION {
        // SAFETY: the copy location borrows this resource.
        pResource: unsafe { std::mem::transmute_copy(r) },
        Type: D3D12_TEXTURE_COPY_TYPE_PLACED_FOOTPRINT,
        Anonymous: D3D12_TEXTURE_COPY_LOCATION_0 {
            PlacedFootprint: D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
                Offset: 0,
                Footprint: D3D12_SUBRESOURCE_FOOTPRINT {
                    Format: RGBA8,
                    Width: 5,
                    Height: 3,
                    Depth: 1,
                    RowPitch: 256,
                },
            },
        },
    };
    let uav = D3D12_RESOURCE_STATE_UNORDERED_ACCESS;
    let rtv = D3D12_RESOURCE_STATE_RENDER_TARGET;
    let (width, height) = if padded { (4, 3) } else { (3, 2) };
    let scene = texture(width, height, D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET, rtv);
    let ui = texture(5, 3, D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET, rtv);
    let target = texture(
        5,
        3,
        D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET | D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS,
        uav,
    );
    let readback = host.read_back_buffer(768).unwrap();
    let commands = host.new_list().unwrap();
    let list = &commands.list;
    // SAFETY: both views and all resources live through command completion.
    unsafe {
        let scene_view = host.rtv(Some(&scene), None);
        let ui_view = host.rtv(Some(&ui), None);
        list.ClearRenderTargetView(scene_view, &[1.0, 0.0, 0.0, 1.0], None);
        let drawn = RECT {
            left: 0,
            right: 3,
            top: 0,
            bottom: 2,
        };
        list.ClearRenderTargetView(scene_view, &[0.0, 0.0, 1.0, 1.0], Some(&[drawn]));
        list.ClearRenderTargetView(ui_view, &[0.0; 4], None);
        let strip = |left, right| RECT {
            left,
            right,
            top: 0,
            bottom: 3,
        };
        if !padded {
            list.ClearRenderTargetView(
                ui_view,
                &[0.0, 128.0 / 255.0, 0.0, 128.0 / 255.0],
                Some(&[strip(2, 3)]),
            );
            list.ClearRenderTargetView(ui_view, &[1.0, 0.0, 0.0, 1.0], Some(&[strip(3, 5)]));
        }
        list.ResourceBarrier(&[
            transition(&scene, rtv, D3D12_RESOURCE_STATE_NON_PIXEL_SHADER_RESOURCE),
            transition(&ui, rtv, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE),
        ]);
        assert!(record_stretch(&host.device, list, &target, &scene, [3, 2]));
        list.ResourceBarrier(&[transition(&target, uav, rtv)]);
        assert!(record(
            &host.device,
            list,
            &target,
            RGBA8,
            (5, 3),
            &ui,
            RGBA8
        ));
        list.ResourceBarrier(&[transition(&target, rtv, D3D12_RESOURCE_STATE_COPY_SOURCE)]);
        list.CopyTextureRegion(&rows(&readback), 0, 0, 0, &sub(&target), None);
    }
    commands.close();
    host.execute(&[&commands]);
    // SAFETY: execute waited for the copy. Each row contains five texels.
    let mut ptr = std::ptr::null_mut();
    let mut pixels = [[0u8; 20]; 3];
    unsafe {
        readback.Map(0, None, Some(&mut ptr)).unwrap();
        for (y, row) in pixels.iter_mut().enumerate() {
            row.copy_from_slice(std::slice::from_raw_parts(
                ptr.cast::<u8>().add(y * 256),
                20,
            ));
        }
        readback.Unmap(0, None);
    }
    Some(pixels)
}

#[test]
fn stretch_and_overlay_preserve_the_expected_pixels() {
    let Some(pixels) = stretched(false) else {
        return;
    };
    let expected = [
        [0, 0, 255, 255],
        [0, 0, 255, 255],
        [0, 128, 127, 255],
        [255, 0, 0, 255],
        [255, 0, 0, 255],
    ];
    for row in pixels {
        assert_eq!(&row[..], expected.as_flattened());
    }
}

/// The title's colour is the top left of a larger texture: what is beside
/// the rendered size must not be filtered into the right and bottom edges.
#[test]
fn stretch_does_not_filter_in_what_is_beside_the_rendered_size() {
    let Some(pixels) = stretched(true) else {
        return;
    };
    for row in pixels {
        assert_eq!(&row[..], [[0u8, 0, 255, 255]; 5].as_flattened());
    }
}
