@react.component
let make = () => {
  let editorContext = EditorContext.useEditorContext()
  let (player, dispatch) = editorContext.usePlayer()

  let handleZoomIn = Hooks.useEvent(_ => {
    let zoomFactor = ZoomUtils.calculateZoomFactorFromDelta(
      -1.0,
      ~currentZoom=player.zoom,
      ~sensitivity=1.0,
      (),
    )
    let newZoom = player.zoom *. zoomFactor

    // Need access to timeline size for proper viewport calculation
    // For now, just use simple zoom without viewport adjustment
    dispatch(Player.SetZoom(newZoom))
  })

  let handleZoomOut = Hooks.useEvent(_ => {
    let zoomFactor = ZoomUtils.calculateZoomFactorFromDelta(
      1.0,
      ~currentZoom=player.zoom,
      ~sensitivity=1.0,
      (),
    )
    let newZoom = player.zoom *. zoomFactor

    // If zooming out to 100% or less, reset viewport to show entire timeline
    if newZoom <= 1.0 {
      dispatch(Player.BatchZoomUpdate(newZoom, 0.0))
    } else {
      // For button-based zoom, just use simple zoom without viewport adjustment
      dispatch(Player.SetZoom(newZoom))
    }
  })

  let handleZoomReset = Hooks.useEvent(_ => {
    dispatch(Player.BatchZoomUpdate(1., 0.0))
  })

  <div className="flex gap-2">
    <button
      onClick=handleZoomOut
      className="text-white hover:bg-gray-700 rounded px-2 py-1 text-sm"
      title="Zoom Out (Ctrl+wheel for fine)">
      {React.string("-")}
    </button>
    <button
      onClick=handleZoomReset
      className="text-white hover:bg-gray-700 rounded px-2 py-1 text-xs font-mono"
      title="Reset Zoom (Fit to timeline)">
      {React.string(`${(player.zoom *. 100.0)->Js.Float.toFixedWithPrecision(~digits=0)}%`)}
    </button>
    <button
      onClick=handleZoomIn
      className="text-white hover:bg-gray-700 rounded px-2 py-1 text-sm"
      title="Zoom In (Shift+wheel for coarse)">
      {React.string("+")}
    </button>
  </div>
}
