;| 3DCtrLine - draws a centerline on layer CtrLine for each selected
   3DSOLID that is a right circular cylinder (including extruded circles
   that were later sliced into sections).
|;

(vl-load-com)

;; Convert a safearray/variant into a plain list
(defun 3dc:to-list (listlike)
  (cond
    ((= (type listlike) 'VARIANT) (vlax-safearray->list (vlax-variant-value listlike)))
    ((= (type listlike) 'SAFEARRAY) (vlax-safearray->list listlike))
    (t listlike)
  )
)

;; vlax-get that returns nil instead of raising an error
(defun 3dc:get (object property / result)
  (setq result (vl-catch-all-apply 'vlax-get (list object property)))
  (if (vl-catch-all-error-p result) nil result)
)

;; Relative difference of two numbers
(defun 3dc:distance (a b)
  ; 1e-12 is to prevent division by 0. Won't matter if a and b are both 0
  (/ (abs (- a b)) (max (abs a) (abs b) 1e-12))
)

;; Create the layer if missing (entmake on a missing layer fails silently)
(defun 3dc:layer (name color)
  (or (tblsearch "LAYER" name)
      (entmake (list '(0 . "LAYER")
                     '(100 . "AcDbSymbolTableRecord")
                     '(100 . "AcDbLayerTableRecord")
                     (cons 2 name)
                     '(70 . 0)
                     (cons 62 color)
                     '(6 . "Continuous")))
  )
)

(defun 3dc:line (p1 p2)
  (entmake (list '(0 . "LINE") '(8 . "CtrLine") (cons 10 p1) (cons 11 p2)))
)

;; Move an object to a layer. Returns T on success, nil if it could not be moved
;; (for example, if the object's current layer is locked).
(defun 3dc:move (object layer / result)
  (setq result (vl-catch-all-apply
            'vla-put-Layer
            (list (vlax-ename->vla-object object) layer)))
  (not (vl-catch-all-error-p result))
)

;; History-free axis detection for a cylinder.
;; Returns (p1 p2) on success, or a string describing why it was rejected.
(defun 3dc:axis (vlaobj ctrd / vol pm pd tol d12 d13 d23 axis i-trans ka2 kt2 r2 h2 h ax len)
  (setq vol   (3dc:get vlaobj 'Volume)
        pm  (3dc:to-list (3dc:get vlaobj 'PrincipalMoments))
        pd  (3dc:to-list (3dc:get vlaobj 'PrincipalDirections))
        tol 0.005
  )
  (cond
    ((not (and vol pm pd (= (length pm) 3) (= (length pd) 9) (> vol 0.0)))
     "could not read mass properties")
    (t
     (setq d12 (3dc:distance (nth 0 pm) (nth 1 pm))
           d13 (3dc:distance (nth 0 pm) (nth 2 pm))
           d23 (3dc:distance (nth 1 pm) (nth 2 pm))
           axis  (cond
                 ((and (< d12 tol) (< d13 tol)) nil) ;; all equal: ambiguous
                 ; Only one direction should have a non-zero distance.
                 ((< d12 tol) 2) ; 1-2 distance is 0, axis is 3
                 ((< d13 tol) 1) ; 1-3 distance is 0, axis is 2
                 ((< d23 tol) 0) ; 2-3 distacne is 0, acis is 1
               )
     )
     (if (not axis)
       "not a right cylinder (principal moments do not fit)"
       (progn
         ;; k^2 = I / V.  Axial: r^2/2.  Transverse: r^2/4 + h^2/12
         (setq i-trans  (/ (- (apply '+ pm) (nth axis pm)) 2.0)
               ka2 (/ (nth axis pm) vol)
               kt2 (/ i-trans vol)
               r2  (* 2.0 ka2)
               h2  (* 12.0 (- kt2 (/ ka2 2.0)))
         )
         (if (or (<= r2 0.0) (<= h2 0.0))
           "not a right cylinder (inconsistent moments)"
           (progn
             (setq h (sqrt h2))
             ;; cross-check: volume must equal pi * r^2 * h
             (if (> (3dc:distance vol (* pi r2 h)) 0.01)
               "not a solid cylinder (volume check failed; cut at an angle or has holes?)"
               (progn
                 (setq ax  (list (nth (* 3 axis) pd)
                                 (nth (+ 1 (* 3 axis)) pd)
                                 (nth (+ 2 (* 3 axis)) pd))
                       len (sqrt (apply '+ (mapcar '* ax ax)))
                 )
                 (if (< len 1e-9)
                   "zero-length axis vector"
                   (progn
                     (setq ax (mapcar '(lambda (x) (* (/ x len) (/ h 2.0))) ax))
                     (list (mapcar '+ ctrd ax) (mapcar '- ctrd ax))
                   )
                 )
               )
             )
           )
         )
       )
     )
    )
  )
)

;; Process one solid. Returns T if a line was made, otherwise a reason string.
(defun 3dc:do (obj / vlaobj ctrd typ pos pos1 tmp res)
  (setq vlaobj (vlax-ename->vla-object obj)
        ctrd   (3dc:to-list (3dc:get vlaobj 'Centroid))
        typ    (3dc:get vlaobj 'SolidType)
  )
  (cond
    ((not ctrd) "could not read centroid")
    ;; Untouched cylinder primitive: exact answer from Position
    ((and (eq typ "Cylinder") (setq pos (3dc:to-list (3dc:get vlaobj 'Position))))
     (setq tmp  (mapcar '- pos ctrd)
           pos1 (mapcar '- ctrd tmp)
     )
     (if (3dc:line pos pos1) T "entmake failed")
    )
    ;; Everything else (extruded circles, sliced sections, etc.)
    (t
     (setq res (3dc:axis vlaobj ctrd))
     (if (listp res)
       (if (3dc:line (car res) (cadr res)) T "entmake failed")
       res
     )
    )
  )
)

;; Create a centerline along a cylinder.
(defun c:3DCtrLine (/ ss itm num res ok bad nomove ent)
  (vl-load-com)
  (prompt "\nSelect 3D Solids to draw Centerlines: ")
  (if (setq ss (ssget '((0 . "3DSOLID"))))
    (progn
      (3dc:layer "CtrLine" 1)
      (3dc:layer "centerlined" 3)
      (3dc:layer "errors" 6)
      (setq itm 0
            num (sslength ss)
            ok  0
            bad 0
            nomove 0
      )
      (while (< itm num)
        (setq ent (ssname ss itm)
              res (vl-catch-all-apply '3dc:do (list ent))
        )
        (cond
          ((vl-catch-all-error-p res)
           (setq bad (1+ bad))
           (prompt (strcat "\n  Solid " (itoa (1+ itm)) " skipped: "
                           (vl-catch-all-error-message res)))
           (if (not (3dc:move ent "errors")) (setq nomove (1+ nomove)))
          )
          ((eq res T)
           (setq ok (1+ ok))
           (if (not (3dc:move ent "centerlined")) (setq nomove (1+ nomove)))
          )
          (t
           (setq bad (1+ bad))
           (prompt (strcat "\n  Solid " (itoa (1+ itm)) " skipped: " res))
           (if (not (3dc:move ent "errors")) (setq nomove (1+ nomove)))
          )
        )
        (setq itm (1+ itm))
      )
      (prompt (strcat "\n" (itoa ok) " centerline(s) created, " (itoa bad) " skipped."))
      (if (> nomove 0)
        (prompt (strcat "\n" (itoa nomove) " solid(s) could not be moved to a new layer (locked layer?)."))
      )
    )
  )
  (princ)
)

(princ "\n3DCtrLine loaded. Type 3DCTRLINE to run.")
(princ)
