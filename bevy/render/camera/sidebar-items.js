window.SIDEBAR_ITEMS = {"enum":[["CameraOutputMode","Control how this camera outputs once rendering is completed."],["NormalizedRenderTarget","Normalized version of the render target."],["Projection","A configurable [`CameraProjection`] that can select its projection type at runtime."],["RenderTarget","The “target” that a [`Camera`] will render to. For example, this could be a `Window` swapchain or an [`Image`]."],["ScalingMode",""]],"fn":[["camera_system","System in charge of updating a [`Camera`] when its window or projection changes."],["extract_cameras",""],["sort_cameras",""]],"struct":[["Camera","The defining component for camera entities, storing information about how and what to render through this camera."],["CameraDriverNode",""],["CameraPlugin",""],["CameraProjectionPlugin","Adds `Camera` driver systems for a given projection type."],["CameraRenderGraph","Configures the `RenderGraph` name assigned to be run for a given [`Camera`] entity."],["CameraUpdateSystem","Label for `camera_system<T>`, shared across all `T`."],["ComputedCameraValues","Holds internally computed [`Camera`] values."],["ExtractedCamera",""],["OrthographicProjection","Project a 3D space onto a 2D surface using parallel lines, i.e., unlike [`PerspectiveProjection`], the size of objects remains the same regardless of their distance to the camera."],["PerspectiveProjection","A 3D camera projection in which distant objects appear smaller than close objects."],["RenderTargetInfo","Information about the current [`RenderTarget`]."],["SortedCamera",""],["SortedCameras","Cameras sorted by their order field. This is updated in the [`sort_cameras`] system."],["Viewport","Render viewport configuration for the [`Camera`] component."]],"trait":[["CameraProjection","Trait to control the projection matrix of a camera."]]};