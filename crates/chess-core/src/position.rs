//! Board position and legal move generation.
//!
//! Representation is a simple 64-square mailbox. Move generation is pseudo-legal
//! generation followed by a king-safety filter (make the move on a clone, reject
//! if our king is attacked). This is not the fastest approach, but it is compact,
//! obviously correct, and validated by perft in the test suite — which is exactly
//! what we want for a low-power e-ink target where clarity beats raw speed.

use crate::moves::{Move, MoveFlag};
use crate::types::{Color, Piece, PieceKind, Square};

/// Castling availability for both sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CastlingRights {
    pub white_kingside: bool,
    pub white_queenside: bool,
    pub black_kingside: bool,
    pub black_queenside: bool,
}

impl CastlingRights {
    pub fn none() -> CastlingRights {
        CastlingRights {
            white_kingside: false,
            white_queenside: false,
            black_kingside: false,
            black_queenside: false,
        }
    }
}

/// A complete, self-contained board state. Cheap to clone (it's a fixed array
/// plus a few scalars), which the move generator relies on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    squares: [Option<Piece>; 64],
    pub side_to_move: Color,
    pub castling: CastlingRights,
    /// En-passant target square (the square a capturing pawn would move *to*).
    pub en_passant: Option<Square>,
    pub halfmove_clock: u32,
    pub fullmove_number: u32,
}

impl Position {
    /// The standard chess starting position.
    pub fn start() -> Position {
        crate::fen::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .expect("valid start FEN")
    }

    /// An empty board (white to move, no rights). Useful for tests and setups.
    pub fn empty() -> Position {
        Position {
            squares: [None; 64],
            side_to_move: Color::White,
            castling: CastlingRights::none(),
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
        }
    }

    #[inline]
    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        self.squares[sq.index()]
    }

    #[inline]
    pub fn set_piece(&mut self, sq: Square, piece: Option<Piece>) {
        self.squares[sq.index()] = piece;
    }

    /// Locate the king of the given color. `None` only on malformed positions.
    pub fn king_square(&self, color: Color) -> Option<Square> {
        for i in 0..64u8 {
            let sq = Square(i);
            if let Some(p) = self.piece_at(sq) {
                if p.color == color && p.kind == PieceKind::King {
                    return Some(sq);
                }
            }
        }
        None
    }

    /// Is `sq` attacked by any piece of `by` color? (Ignores whose turn it is.)
    pub fn is_attacked(&self, sq: Square, by: Color) -> bool {
        let (f, r) = (sq.file(), sq.rank());

        // Pawn attacks: a pawn of `by` attacks `sq` if it sits one rank *behind*
        // sq (relative to its own advance direction) on an adjacent file.
        let pdir = by.pawn_dir();
        for df in [-1, 1] {
            if let Some(from) = Square::from_file_rank(f + df, r - pdir) {
                if self.piece_at(from) == Some(Piece::new(by, PieceKind::Pawn)) {
                    return true;
                }
            }
        }

        // Knight attacks.
        const KNIGHT: [(i8, i8); 8] = [
            (1, 2),
            (2, 1),
            (2, -1),
            (1, -2),
            (-1, -2),
            (-2, -1),
            (-2, 1),
            (-1, 2),
        ];
        for (df, dr) in KNIGHT {
            if let Some(from) = Square::from_file_rank(f + df, r + dr) {
                if self.piece_at(from) == Some(Piece::new(by, PieceKind::Knight)) {
                    return true;
                }
            }
        }

        // King attacks (adjacent squares).
        for df in -1..=1 {
            for dr in -1..=1 {
                if df == 0 && dr == 0 {
                    continue;
                }
                if let Some(from) = Square::from_file_rank(f + df, r + dr) {
                    if self.piece_at(from) == Some(Piece::new(by, PieceKind::King)) {
                        return true;
                    }
                }
            }
        }

        // Sliding attacks: rook/queen orthogonally, bishop/queen diagonally.
        const ORTHO: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
        const DIAG: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
        if self.ray_hits(f, r, &ORTHO, by, PieceKind::Rook) {
            return true;
        }
        if self.ray_hits(f, r, &DIAG, by, PieceKind::Bishop) {
            return true;
        }
        false
    }

    /// Walk each ray from (f,r); return true if the first piece encountered is
    /// `by`-colored and is either `slider` or a queen.
    fn ray_hits(&self, f: i8, r: i8, dirs: &[(i8, i8)], by: Color, slider: PieceKind) -> bool {
        for &(df, dr) in dirs {
            let mut cf = f + df;
            let mut cr = r + dr;
            while let Some(sq) = Square::from_file_rank(cf, cr) {
                if let Some(p) = self.piece_at(sq) {
                    if p.color == by && (p.kind == slider || p.kind == PieceKind::Queen) {
                        return true;
                    }
                    break; // blocked by some piece
                }
                cf += df;
                cr += dr;
            }
        }
        false
    }

    /// Is the side to move currently in check?
    pub fn is_in_check(&self) -> bool {
        match self.king_square(self.side_to_move) {
            Some(k) => self.is_attacked(k, self.side_to_move.opponent()),
            None => false,
        }
    }

    /// All fully legal moves for the side to move.
    pub fn legal_moves(&self) -> Vec<Move> {
        let mut out = Vec::with_capacity(48);
        let us = self.side_to_move;
        for mv in self.pseudo_legal_moves() {
            let next = self.apply_unchecked(mv);
            // After our move it's the opponent's turn; verify *our* king is safe.
            if let Some(k) = next.king_square(us) {
                if !next.is_attacked(k, us.opponent()) {
                    out.push(mv);
                }
            }
        }
        out
    }

    /// Pseudo-legal moves (may leave own king in check; castling through/into
    /// check is filtered here, but pins are handled by `legal_moves`).
    fn pseudo_legal_moves(&self) -> Vec<Move> {
        let mut moves = Vec::with_capacity(48);
        let us = self.side_to_move;
        for i in 0..64u8 {
            let sq = Square(i);
            let piece = match self.piece_at(sq) {
                Some(p) if p.color == us => p,
                _ => continue,
            };
            match piece.kind {
                PieceKind::Pawn => self.gen_pawn(sq, us, &mut moves),
                PieceKind::Knight => self.gen_steps(sq, us, &KNIGHT_STEPS, &mut moves),
                PieceKind::King => {
                    self.gen_steps(sq, us, &KING_STEPS, &mut moves);
                    self.gen_castles(us, &mut moves);
                }
                PieceKind::Bishop => self.gen_slides(sq, us, &DIAG_DIRS, &mut moves),
                PieceKind::Rook => self.gen_slides(sq, us, &ORTHO_DIRS, &mut moves),
                PieceKind::Queen => {
                    self.gen_slides(sq, us, &DIAG_DIRS, &mut moves);
                    self.gen_slides(sq, us, &ORTHO_DIRS, &mut moves);
                }
            }
        }
        moves
    }

    fn gen_steps(&self, from: Square, us: Color, steps: &[(i8, i8)], out: &mut Vec<Move>) {
        for &(df, dr) in steps {
            if let Some(to) = Square::from_file_rank(from.file() + df, from.rank() + dr) {
                match self.piece_at(to) {
                    Some(p) if p.color == us => {}
                    _ => out.push(Move::normal(from, to)),
                }
            }
        }
    }

    fn gen_slides(&self, from: Square, us: Color, dirs: &[(i8, i8)], out: &mut Vec<Move>) {
        for &(df, dr) in dirs {
            let mut f = from.file() + df;
            let mut r = from.rank() + dr;
            while let Some(to) = Square::from_file_rank(f, r) {
                match self.piece_at(to) {
                    Some(p) if p.color == us => break,
                    Some(_) => {
                        out.push(Move::normal(from, to));
                        break;
                    }
                    None => out.push(Move::normal(from, to)),
                }
                f += df;
                r += dr;
            }
        }
    }

    fn gen_pawn(&self, from: Square, us: Color, out: &mut Vec<Move>) {
        let dir = us.pawn_dir();
        let start_rank = if us == Color::White { 1 } else { 6 };
        let promo_rank = if us == Color::White { 7 } else { 0 };
        let (f, r) = (from.file(), from.rank());

        // Single push.
        if let Some(one) = Square::from_file_rank(f, r + dir) {
            if self.piece_at(one).is_none() {
                self.push_pawn_move(from, one, promo_rank, MoveFlag::Normal, out);
                // Double push from the starting rank.
                if r == start_rank {
                    if let Some(two) = Square::from_file_rank(f, r + 2 * dir) {
                        if self.piece_at(two).is_none() {
                            out.push(Move {
                                from,
                                to: two,
                                promotion: None,
                                flag: MoveFlag::DoublePawnPush,
                            });
                        }
                    }
                }
            }
        }

        // Captures (including en passant).
        for df in [-1, 1] {
            if let Some(to) = Square::from_file_rank(f + df, r + dir) {
                if let Some(p) = self.piece_at(to) {
                    if p.color != us {
                        self.push_pawn_move(from, to, promo_rank, MoveFlag::Normal, out);
                    }
                } else if self.en_passant == Some(to) {
                    out.push(Move {
                        from,
                        to,
                        promotion: None,
                        flag: MoveFlag::EnPassant,
                    });
                }
            }
        }
    }

    fn push_pawn_move(
        &self,
        from: Square,
        to: Square,
        promo_rank: i8,
        flag: MoveFlag,
        out: &mut Vec<Move>,
    ) {
        if to.rank() == promo_rank {
            for kind in [
                PieceKind::Queen,
                PieceKind::Rook,
                PieceKind::Bishop,
                PieceKind::Knight,
            ] {
                out.push(Move {
                    from,
                    to,
                    promotion: Some(kind),
                    flag,
                });
            }
        } else {
            out.push(Move {
                from,
                to,
                promotion: None,
                flag,
            });
        }
    }

    fn gen_castles(&self, us: Color, out: &mut Vec<Move>) {
        // Can't castle out of check.
        let king_from = match self.king_square(us) {
            Some(k) => k,
            None => return,
        };
        let opp = us.opponent();
        if self.is_attacked(king_from, opp) {
            return;
        }
        let (rank, ks, qs) = match us {
            Color::White => (0, self.castling.white_kingside, self.castling.white_queenside),
            Color::Black => (7, self.castling.black_kingside, self.castling.black_queenside),
        };
        // King always starts on e-file for standard castling.
        let e = Square::from_file_rank(4, rank).unwrap();
        if king_from != e {
            return;
        }

        // Kingside: f and g empty, king doesn't pass through/into attack.
        if ks {
            let fsq = Square::from_file_rank(5, rank).unwrap();
            let gsq = Square::from_file_rank(6, rank).unwrap();
            if self.piece_at(fsq).is_none()
                && self.piece_at(gsq).is_none()
                && !self.is_attacked(fsq, opp)
                && !self.is_attacked(gsq, opp)
            {
                out.push(Move {
                    from: e,
                    to: gsq,
                    promotion: None,
                    flag: MoveFlag::CastleKingside,
                });
            }
        }
        // Queenside: b, c, d empty; king passes over d and c.
        if qs {
            let dsq = Square::from_file_rank(3, rank).unwrap();
            let csq = Square::from_file_rank(2, rank).unwrap();
            let bsq = Square::from_file_rank(1, rank).unwrap();
            if self.piece_at(dsq).is_none()
                && self.piece_at(csq).is_none()
                && self.piece_at(bsq).is_none()
                && !self.is_attacked(dsq, opp)
                && !self.is_attacked(csq, opp)
            {
                out.push(Move {
                    from: e,
                    to: csq,
                    promotion: None,
                    flag: MoveFlag::CastleQueenside,
                });
            }
        }
    }

    /// Apply a move without checking legality, returning the resulting position.
    /// Assumes `mv` came from this position's generator (or is otherwise valid).
    pub fn apply_unchecked(&self, mv: Move) -> Position {
        let mut next = self.clone();
        let us = self.side_to_move;
        let mut piece = self.piece_at(mv.from).expect("move from empty square");
        let is_capture = self.piece_at(mv.to).is_some() || mv.flag == MoveFlag::EnPassant;
        let is_pawn = piece.kind == PieceKind::Pawn;

        next.set_piece(mv.from, None);

        // En-passant removes the passed pawn, which sits beside the destination.
        if mv.flag == MoveFlag::EnPassant {
            let cap_rank = mv.from.rank();
            let cap_sq = Square::from_file_rank(mv.to.file(), cap_rank).unwrap();
            next.set_piece(cap_sq, None);
        }

        // Promotion swaps the pawn for the chosen piece.
        if let Some(kind) = mv.promotion {
            piece = Piece::new(us, kind);
        }
        next.set_piece(mv.to, Some(piece));

        // Move the rook when castling.
        match mv.flag {
            MoveFlag::CastleKingside => {
                let rank = mv.from.rank();
                let rook_from = Square::from_file_rank(7, rank).unwrap();
                let rook_to = Square::from_file_rank(5, rank).unwrap();
                let rook = next.piece_at(rook_from);
                next.set_piece(rook_from, None);
                next.set_piece(rook_to, rook);
            }
            MoveFlag::CastleQueenside => {
                let rank = mv.from.rank();
                let rook_from = Square::from_file_rank(0, rank).unwrap();
                let rook_to = Square::from_file_rank(3, rank).unwrap();
                let rook = next.piece_at(rook_from);
                next.set_piece(rook_from, None);
                next.set_piece(rook_to, rook);
            }
            _ => {}
        }

        // Update castling rights: king move revokes both sides; a rook leaving
        // its home square (moved or captured) revokes that side.
        if piece.kind == PieceKind::King {
            match us {
                Color::White => {
                    next.castling.white_kingside = false;
                    next.castling.white_queenside = false;
                }
                Color::Black => {
                    next.castling.black_kingside = false;
                    next.castling.black_queenside = false;
                }
            }
        }
        // Any move touching a corner square (as origin or destination) clears the
        // matching right — covers the rook moving away and the rook being captured.
        for sq in [mv.from, mv.to] {
            match (sq.file(), sq.rank()) {
                (0, 0) => next.castling.white_queenside = false,
                (7, 0) => next.castling.white_kingside = false,
                (0, 7) => next.castling.black_queenside = false,
                (7, 7) => next.castling.black_kingside = false,
                _ => {}
            }
        }

        // En-passant target only after a double push.
        next.en_passant = if mv.flag == MoveFlag::DoublePawnPush {
            let mid_rank = (mv.from.rank() + mv.to.rank()) / 2;
            Square::from_file_rank(mv.from.file(), mid_rank)
        } else {
            None
        };

        // Halfmove clock: reset on pawn move or capture, else increment.
        next.halfmove_clock = if is_pawn || is_capture {
            0
        } else {
            self.halfmove_clock + 1
        };
        if us == Color::Black {
            next.fullmove_number = self.fullmove_number + 1;
        }
        next.side_to_move = us.opponent();
        next
    }
}

const KNIGHT_STEPS: [(i8, i8); 8] = [
    (1, 2),
    (2, 1),
    (2, -1),
    (1, -2),
    (-1, -2),
    (-2, -1),
    (-2, 1),
    (-1, 2),
];
const KING_STEPS: [(i8, i8); 8] = [
    (1, 0),
    (-1, 0),
    (0, 1),
    (0, -1),
    (1, 1),
    (1, -1),
    (-1, 1),
    (-1, -1),
];
const ORTHO_DIRS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
const DIAG_DIRS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
