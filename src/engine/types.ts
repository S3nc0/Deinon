export interface Point {
  x: number;
  y: number;
}

/** The 4 points, clockwise: top left, top right, bottom right, bottom left. */
export type Quad = Point[];

/** A 3 × 3 homography in row order. */
export type Matrix = number[];

export interface Camera {
  yaw: number;
  pitch: number;
  roll: number;
  fov: number;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}
